use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_title("Toonvara"),
        ..Default::default()
    };

    eframe::run_native(
        "Toonvara",
        options,
        Box::new(|_cc| Ok(Box::new(ToonvaraApp))),
    )
}

#[derive(Default)]
struct ToonvaraApp;

impl eframe::App for ToonvaraApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Toonvara");
            ui.separator();
            ui.label("Cartoon video generator — desktop shell");
            ui.add_space(16.0);
            ui.label("The viewport and editing tools will be built in later phases.");
            ui.add_space(8.0);
            ui.weak(format!("studio-core v{}", env!("CARGO_PKG_VERSION")));
        });
    }
}
