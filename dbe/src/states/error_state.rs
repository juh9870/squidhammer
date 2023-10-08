use crate::states::DbeStateHolder;
use crate::{info_window, DbeState};
use derivative::Derivative;
use egui::{ScrollArea, Ui};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use utils::errors::display_error;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct ErrorState {
    err: String,
    #[derivative(Debug = "ignore")]
    cache: CommonMarkCache,
}

impl ErrorState {
    pub fn new(err: anyhow::Error) -> Self {
        Self {
            err: display_error(err),
            cache: Default::default(),
        }
    }
}

impl DbeStateHolder for ErrorState {
    fn update(mut self, ui: &mut Ui) -> DbeState {
        info_window(ui, "Something gone wrong", |ui| {
            ui.set_max_height(ui.ctx().available_rect().height() * 0.9 - 64.);
            ui.set_min_width(ui.ctx().available_rect().width() * 0.9 - 64.);
            ScrollArea::vertical().show(ui, |ui| {
                CommonMarkViewer::new("error_viewer").show(ui, &mut self.cache, &self.err)
            });
        });
        self.into()
    }
}
