use std::path::{Path, PathBuf};
use std::sync::mpsc;

use eframe::egui;
use studio_core::Project;
use studio_render::pipeline::{PipelineError, VideoPipelineParams, VideoPipelineResult};

fn main() -> eframe::Result {
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../../../public/favicon.png"))
        .map_err(|error| eframe::Error::AppCreation(Box::new(error)))?;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_title("Toonvara")
            .with_icon(icon.clone()),
        ..Default::default()
    };

    eframe::run_native(
        "Toonvara",
        options,
        Box::new(move |cc| Ok(Box::new(ToonvaraApp::new(cc, &icon)))),
    )
}

// ---------------------------------------------------------------------------
// Messages sent from the render thread to the UI
// ---------------------------------------------------------------------------

enum RenderMsg {
    Status(String),
    Done(Box<VideoPipelineResult>),
    Failed(String),
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct ToonvaraApp {
    logo: egui::TextureHandle,
    about_open: bool,
    loaded: Option<LoadedProject>,
    load_error: Option<String>,
    render_state: RenderState,
}

struct LoadedProject {
    path: PathBuf,
    project: Project,
}

enum RenderState {
    Idle,
    Running {
        rx: mpsc::Receiver<RenderMsg>,
        status: String,
        log_lines: Vec<String>,
    },
    Done(RenderSuccess),
    Failed(String),
}

struct RenderSuccess {
    output_path: PathBuf,
    size_kb: f64,
    duration_secs: f64,
    width: u32,
    height: u32,
    fps: f64,
}

impl ToonvaraApp {
    fn new(cc: &eframe::CreationContext<'_>, icon: &egui::IconData) -> Self {
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [
                usize::try_from(icon.width).expect("logo width fits usize"),
                usize::try_from(icon.height).expect("logo height fits usize"),
            ],
            &icon.rgba,
        );
        Self {
            logo: cc
                .egui_ctx
                .load_texture("Toonvara logo", image, egui::TextureOptions::LINEAR),
            about_open: false,
            loaded: None,
            load_error: None,
            render_state: RenderState::Idle,
        }
    }

    fn open_project(&mut self) {
        let path = rfd::FileDialog::new()
            .add_filter("Toonvara JSON", &["json"])
            .set_title("Open Toonvara project")
            .pick_file();

        let Some(path) = path else { return };

        match studio_json::parse_file(&path) {
            Ok(project) => {
                self.loaded = Some(LoadedProject { path, project });
                self.load_error = None;
                self.render_state = RenderState::Idle;
            }
            Err(e) => {
                self.load_error = Some(format!("{}: {e}", path.display()));
                self.loaded = None;
            }
        }
    }

    fn is_rendering(&self) -> bool {
        matches!(self.render_state, RenderState::Running { .. })
    }

    fn has_output(&self) -> bool {
        matches!(self.render_state, RenderState::Done(_))
    }

    fn start_render(&mut self, ctx: &egui::Context) {
        let Some(loaded) = &self.loaded else { return };

        let default_out = PathBuf::from(&loaded.project.output.path);
        let default_name = default_out.file_name().map_or_else(
            || "output.mp4".to_string(),
            |n| n.to_string_lossy().into_owned(),
        );

        let out_path = rfd::FileDialog::new()
            .add_filter("MP4 video", &["mp4"])
            .set_file_name(&default_name)
            .set_title("Save rendered video")
            .save_file();

        let Some(out_path) = out_path else { return };

        let project = &loaded.project;
        let fps = project.video.fps;
        let width = project.video.width;
        let height = project.video.height;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let frame_count = (project.video.duration_seconds * f64::from(fps)).round() as u32;

        let (cam_pos, cam_tgt) = project
            .scenes
            .first()
            .and_then(|s| s.camera.as_ref())
            .map_or(([0.0, 2.0, 6.0], [0.0, 1.0, 0.0]), |c| {
                (
                    [c.position.x, c.position.y, c.position.z],
                    [c.target.x, c.target.y, c.target.z],
                )
            });

        let json_dir = loaded.path.parent().unwrap_or(Path::new(".")).to_path_buf();

        let scenes = match studio_render::schedule::resolve(project, &json_dir, frame_count) {
            Ok(s) => s,
            Err(e) => {
                self.render_state = RenderState::Failed(format!("Schedule error: {e}"));
                return;
            }
        };

        let script = find_adapter_script("render.py");

        let (tx, rx) = mpsc::channel();
        self.render_state = RenderState::Running {
            rx,
            status: "starting render...".to_string(),
            log_lines: Vec::new(),
        };

        let ctx = ctx.clone();

        std::thread::spawn(move || {
            let send = |msg: RenderMsg| {
                let _ = tx.send(msg);
                ctx.request_repaint();
            };

            send(RenderMsg::Status(format!(
                "rendering {frame_count} frames ({width}x{height} @ {fps} fps)..."
            )));

            let params = VideoPipelineParams {
                blender_script: script,
                out_path: out_path.clone(),
                width,
                height,
                fps,
                frame_count,
                camera_pos: cam_pos,
                camera_target: cam_tgt,
                character_glb: None,
                character_pos: [0.0; 3],
                animation_clip: None,
                scenes: Some(scenes),
            };

            match studio_render::pipeline::render_video(&params) {
                Ok(result) => {
                    for line in result.blender_log.lines() {
                        if line.starts_with("TOONVARA_SCHEDULE:")
                            || line.starts_with("TOONVARA_IMPORT:")
                            || line.starts_with("TOONVARA_RIG:")
                        {
                            send(RenderMsg::Status(line.to_string()));
                        }
                    }
                    send(RenderMsg::Done(Box::new(result)));
                }
                Err(e) => {
                    send(RenderMsg::Failed(format_pipeline_error(&e)));
                }
            }
        });
    }

    fn poll_render(&mut self) {
        let RenderState::Running {
            rx,
            status,
            log_lines,
        } = &mut self.render_state
        else {
            return;
        };

        while let Ok(msg) = rx.try_recv() {
            match msg {
                RenderMsg::Status(line) => {
                    status.clone_from(&line);
                    log_lines.push(line);
                }
                RenderMsg::Done(result) => {
                    #[allow(clippy::cast_precision_loss)]
                    let size_kb = result.output_size as f64 / 1024.0;
                    self.render_state = RenderState::Done(RenderSuccess {
                        output_path: result.output_path.clone(),
                        size_kb,
                        duration_secs: result.probe.duration_secs,
                        width: result.probe.width,
                        height: result.probe.height,
                        fps: result.probe.fps,
                    });
                    return;
                }
                RenderMsg::Failed(err) => {
                    self.render_state = RenderState::Failed(err);
                    return;
                }
            }
        }
    }

    fn open_output(&self) {
        let RenderState::Done(success) = &self.render_state else {
            return;
        };
        open_in_player(&success.output_path);
    }
}

// ---------------------------------------------------------------------------
// UI
// ---------------------------------------------------------------------------

impl eframe::App for ToonvaraApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_render();

        // --- Toolbar -------------------------------------------------------
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.add(egui::Image::new(&self.logo).max_size(egui::vec2(24.0, 24.0)));
                ui.strong("Toonvara");
                ui.separator();

                // Open
                if ui
                    .add_enabled(!self.is_rendering(), egui::Button::new("Open"))
                    .clicked()
                {
                    self.open_project();
                }

                // Render
                let can_render = self.loaded.is_some() && !self.is_rendering();
                if ui
                    .add_enabled(can_render, egui::Button::new("Render"))
                    .clicked()
                {
                    self.start_render(ctx);
                }

                // Open output (only after successful render)
                if self.has_output() && ui.button("Open output").clicked() {
                    self.open_output();
                }

                ui.separator();
                if ui.small_button("About").clicked() {
                    self.about_open = true;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.weak(format!("v{}", env!("CARGO_PKG_VERSION")));
                });
            });
            ui.add_space(2.0);
        });

        // --- Status / progress panel (bottom) ------------------------------
        show_status_panel(ctx, &self.render_state);

        // --- Central panel -------------------------------------------------
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(err) = &self.load_error {
                error_banner(ui, err);
                ui.add_space(8.0);
            }

            if let Some(loaded) = &self.loaded {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    show_project_summary(ui, loaded, &self.render_state);
                });
            } else {
                show_empty_state(ui);
            }
        });

        // --- About dialog --------------------------------------------------
        egui::Window::new("About Toonvara")
            .open(&mut self.about_open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(egui::Image::new(&self.logo).max_size(egui::vec2(120.0, 120.0)));
                    ui.heading("Toonvara");
                    ui.label("Cartoon video generator");
                    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                });
            });
    }
}

// ---------------------------------------------------------------------------
// Empty state
// ---------------------------------------------------------------------------

fn show_empty_state(ui: &mut egui::Ui) {
    ui.add_space(80.0);
    ui.vertical_centered(|ui| {
        ui.heading("Welcome to Toonvara");
        ui.add_space(12.0);
        ui.label("Open a .json project file to get started.");
        ui.add_space(4.0);
        ui.weak("Then click Render to produce an MP4.");
    });
}

// ---------------------------------------------------------------------------
// Status / progress panel (bottom)
// ---------------------------------------------------------------------------

fn show_status_panel(ctx: &egui::Context, state: &RenderState) {
    match state {
        RenderState::Idle => {
            // Minimal status bar even when idle.
            egui::TopBottomPanel::bottom("status_panel").show(ctx, |ui| {
                ui.add_space(2.0);
                ui.weak("Ready");
                ui.add_space(2.0);
            });
        }
        RenderState::Running {
            status, log_lines, ..
        } => {
            egui::TopBottomPanel::bottom("status_panel").show(ctx, |ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.strong(status);
                });
                if log_lines.len() > 1 {
                    ui.weak(&log_lines[log_lines.len() - 2]);
                }
                ui.add_space(4.0);
            });
        }
        RenderState::Done(success) => {
            egui::TopBottomPanel::bottom("status_panel").show(ctx, |ui| {
                ui.add_space(4.0);
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(20, 60, 20))
                    .inner_margin(8.0)
                    .rounding(4.0)
                    .show(ui, |ui| {
                        ui.colored_label(
                            egui::Color32::from_rgb(180, 255, 180),
                            format!("Render complete: {}", success.output_path.display()),
                        );
                        ui.colored_label(
                            egui::Color32::from_rgb(160, 220, 160),
                            format!(
                                "{:.1} KB  |  {}x{}  |  {:.1} fps  |  {:.1}s",
                                success.size_kb,
                                success.width,
                                success.height,
                                success.fps,
                                success.duration_secs,
                            ),
                        );
                    });
                ui.add_space(4.0);
            });
        }
        RenderState::Failed(message) => {
            egui::TopBottomPanel::bottom("status_panel").show(ctx, |ui| {
                ui.add_space(4.0);
                error_banner(ui, message);
                ui.add_space(4.0);
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Shared UI helpers
// ---------------------------------------------------------------------------

fn error_banner(ui: &mut egui::Ui, text: &str) {
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(80, 20, 20))
        .inner_margin(8.0)
        .rounding(4.0)
        .show(ui, |ui| {
            ui.colored_label(egui::Color32::from_rgb(255, 180, 180), text);
        });
}

// ---------------------------------------------------------------------------
// Project summary panel
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_lines)]
fn show_project_summary(ui: &mut egui::Ui, loaded: &LoadedProject, render_state: &RenderState) {
    let p = &loaded.project;

    ui.heading(&p.name);
    ui.weak(loaded.path.display().to_string());
    ui.add_space(4.0);

    // State hint.
    match render_state {
        RenderState::Idle => {
            ui.label("Ready to render. Click Render in the toolbar.");
        }
        RenderState::Running { .. } => {
            ui.label("Rendering in progress...");
        }
        RenderState::Done(_) => {
            ui.label("Render finished. Click Open output to view, or Render again.");
        }
        RenderState::Failed(_) => {
            ui.label("Render failed. Check the error below, fix, and try again.");
        }
    }

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    egui::Grid::new("project_summary")
        .num_columns(2)
        .spacing([16.0, 4.0])
        .show(ui, |ui| {
            row(
                ui,
                "Resolution",
                &format!("{}x{}", p.video.width, p.video.height),
            );
            row(ui, "FPS", &p.video.fps.to_string());
            row(ui, "Duration", &format!("{:.1}s", p.video.duration_seconds));
            row(ui, "Scenes", &p.scenes.len().to_string());
            let total_chars: usize = p.scenes.iter().map(|s| s.characters.len()).sum();
            row(ui, "Characters", &total_chars.to_string());
            let total_audio: usize = p.scenes.iter().map(|s| s.audio.len()).sum();
            row(ui, "Audio tracks", &total_audio.to_string());
            row(
                ui,
                "Output",
                &format!("{} \u{2192} {}", p.output.format, p.output.path),
            );
        });

    if !p.assets.characters.is_empty() {
        ui.add_space(12.0);
        ui.strong("Character Assets");
        ui.add_space(4.0);
        for (id, path) in &p.assets.characters {
            ui.horizontal(|ui| {
                ui.monospace(id);
                ui.weak(path);
            });
        }
    }

    ui.add_space(12.0);
    ui.strong("Scenes");
    ui.add_space(4.0);

    for scene in &p.scenes {
        egui::CollapsingHeader::new(format!(
            "{} (frames {}..{})",
            scene.id,
            scene.start_frame,
            scene.start_frame + scene.duration_frames
        ))
        .default_open(true)
        .show(ui, |ui| {
            if let Some(env) = &scene.environment {
                ui.label(format!("Environment: {env}"));
            }

            for ch in &scene.characters {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.strong(&ch.id);
                    ui.weak(format!("(asset: {})", ch.asset));
                });
                if !ch.actions.is_empty() {
                    egui::Grid::new(format!("actions_{}", ch.id))
                        .num_columns(3)
                        .spacing([12.0, 2.0])
                        .show(ui, |ui| {
                            for a in &ch.actions {
                                ui.label(&a.clip);
                                ui.weak(format!(
                                    "frames {}..{}",
                                    a.start_frame,
                                    a.start_frame + a.duration_frames
                                ));
                                if (a.speed - 1.0).abs() > f64::EPSILON {
                                    ui.weak(format!("{:.1}x", a.speed));
                                } else {
                                    ui.label("");
                                }
                                ui.end_row();
                            }
                        });
                }
            }

            for clip in &scene.audio {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.strong(format!("{:?}", clip.audio_type));
                    ui.weak(&clip.path);
                    ui.weak(format!(
                        "frame {} vol {:.0}%",
                        clip.start_frame,
                        clip.volume * 100.0
                    ));
                });
            }
        });
    }
}

fn row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.strong(label);
    ui.label(value);
    ui.end_row();
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Locate an adapter script relative to CWD or the running binary.
fn find_adapter_script(filename: &str) -> PathBuf {
    let rel = format!("adapters/blender/{filename}");
    let candidates: Vec<PathBuf> = vec![
        PathBuf::from(&rel),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join(format!("../../{rel}"))))
            .unwrap_or_default(),
    ];
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    candidates.into_iter().next().unwrap_or_default()
}

/// Open a file in the system default player.
fn open_in_player(path: &Path) {
    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(target_os = "windows")]
    let cmd = "start";
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let cmd = "xdg-open";

    let _ = std::process::Command::new(cmd)
        .arg(path.as_os_str())
        .spawn();
}

fn format_pipeline_error(err: &PipelineError) -> String {
    match err {
        PipelineError::Blender(inner) => format!("Blender failed: {inner}"),
        PipelineError::Ffmpeg(inner) => format!("FFmpeg failed: {inner}"),
        PipelineError::Io(e) => format!("I/O error: {e}"),
        PipelineError::FrameCountMismatch { expected, found } => {
            format!("Frame count mismatch: expected {expected}, got {found}")
        }
        PipelineError::VerifyFailed { detail } => format!("Verification failed: {detail}"),
        PipelineError::CharacterMissing { path } => {
            format!("Character GLB not found: {path}")
        }
        PipelineError::ClipValidation(detail) => format!("Clip not in GLB: {detail}"),
        PipelineError::AudioMissing { path } => format!("Audio file not found: {path}"),
    }
}
