use std::path::PathBuf;

use eframe::egui;
use studio_core::Project;

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
// App state
// ---------------------------------------------------------------------------

struct ToonvaraApp {
    logo: egui::TextureHandle,
    about_open: bool,
    /// Currently loaded project (if any).
    loaded: Option<LoadedProject>,
    /// Most recent parse error (displayed in the UI).
    load_error: Option<String>,
}

struct LoadedProject {
    path: PathBuf,
    project: Project,
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
            }
            Err(e) => {
                self.load_error = Some(format!("{}: {e}", path.display()));
                self.loaded = None;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// UI
// ---------------------------------------------------------------------------

impl eframe::App for ToonvaraApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- Top bar -------------------------------------------------------
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::Image::new(&self.logo).max_size(egui::vec2(28.0, 28.0)));
                ui.heading("Toonvara");
                ui.separator();
                if ui.button("Open").clicked() {
                    self.open_project();
                }
                if ui.button("About").clicked() {
                    self.about_open = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.weak(format!("v{}", env!("CARGO_PKG_VERSION")));
                });
            });
        });

        // --- Central panel -------------------------------------------------
        egui::CentralPanel::default().show(ctx, |ui| {
            // Parse error banner.
            if let Some(err) = &self.load_error {
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(80, 20, 20))
                    .inner_margin(8.0)
                    .rounding(4.0)
                    .show(ui, |ui| {
                        ui.colored_label(egui::Color32::from_rgb(255, 180, 180), err);
                    });
                ui.add_space(8.0);
            }

            if let Some(loaded) = &self.loaded {
                show_project_summary(ui, loaded);
            } else {
                ui.add_space(40.0);
                ui.vertical_centered(|ui| {
                    ui.label("No project loaded.");
                    ui.add_space(8.0);
                    ui.label("Click Open to load a .json project file.");
                });
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
// Project summary panel
// ---------------------------------------------------------------------------

fn show_project_summary(ui: &mut egui::Ui, loaded: &LoadedProject) {
    let p = &loaded.project;

    ui.heading(&p.name);
    ui.weak(loaded.path.display().to_string());
    ui.add_space(12.0);

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

    // --- Character assets --------------------------------------------------
    if !p.assets.characters.is_empty() {
        ui.add_space(16.0);
        ui.heading("Character Assets");
        ui.add_space(4.0);
        for (id, path) in &p.assets.characters {
            ui.horizontal(|ui| {
                ui.strong(id);
                ui.weak(path);
            });
        }
    }

    // --- Scenes ------------------------------------------------------------
    ui.add_space(16.0);
    ui.heading("Scenes");
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
