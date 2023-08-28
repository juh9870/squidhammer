use dbe::NodeGraphExample;
use eframe::{egui, Error};
fn main() -> Result<(), Error> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Database Editor",
        native_options,
        Box::new(|cc| Box::new(NodeGraphExample::new(cc))),
    )?;
    Ok(())
}

#[derive(Default)]
struct DbeApp {}

impl DbeApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        Self::default()
    }
}

impl eframe::App for DbeApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hello World!");
        });
    }
}
