use dbe::{DbeData, DbeState};
use eframe::{egui, Error};

fn main() -> Result<(), Error> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Database Editor",
        native_options,
        Box::new(|cc| Box::new(DbeApp::new(cc))),
    )?;
    Ok(())
}

#[derive(Default)]
struct DbeApp {
    data: DbeState,
}

impl DbeApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        Self::default()
    }
}

impl eframe::App for DbeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        dbe::update_dbe(ctx, &mut self.data);
    }
}
