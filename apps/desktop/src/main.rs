use eframe::egui;

fn main() -> eframe::Result {
    // Embedded bytes work with cargo run, a standalone binary, and an app bundle.
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../../../public/favicon.png"))
        .map_err(|error| eframe::Error::AppCreation(Box::new(error)))?;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_title("Toonvara")
            // eframe also uses this for NSApplication's Dock icon on macOS.
            .with_icon(icon.clone()),
        ..Default::default()
    };

    eframe::run_native(
        "Toonvara",
        options,
        Box::new(move |cc| Ok(Box::new(ToonvaraApp::new(cc, &icon)))),
    )
}

struct ToonvaraApp {
    logo: egui::TextureHandle,
    about_open: bool,
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
        }
    }
}

impl eframe::App for ToonvaraApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::Image::new(&self.logo).max_size(egui::vec2(40.0, 40.0)));
                ui.heading("Toonvara");
                if ui.button("About Toonvara").clicked() {
                    self.about_open = true;
                }
            });
            ui.separator();
            ui.label("Cartoon video generator — desktop shell");
            ui.add_space(16.0);
            ui.label("The viewport and editing tools will be built in later phases.");
            ui.add_space(8.0);
            ui.weak(format!("studio-core v{}", env!("CARGO_PKG_VERSION")));
        });
        egui::Window::new("About Toonvara")
            .open(&mut self.about_open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(egui::Image::new(&self.logo).max_size(egui::vec2(160.0, 160.0)));
                    ui.heading("Toonvara");
                    ui.label("Cartoon video generator");
                    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                });
            });
    }
}
