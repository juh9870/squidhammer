use crate::widgets::collapsible_toolbar::ToolbarViewer;
use egui::Ui;

pub fn simple_new_tab_menu<Viewer: ToolbarViewer>(
    ui: &mut Ui,
    viewer: &Viewer,
    tabs: &[Viewer::Tab],
) -> Option<Viewer::Tab>
where
    Viewer::Tab: Clone,
{
    let mut selected_tab = None;

    let height =
        ui.text_style_height(&egui::TextStyle::Button) + ui.style().spacing.button_padding.y * 2.0;

    egui::ScrollArea::vertical().show_rows(ui, height, tabs.len(), |ui, range| {
        for tab in &tabs[range] {
            if ui.selectable_label(false, viewer.title(tab)).clicked() {
                selected_tab = Some(tab.clone());
            }
        }
    });

    selected_tab
}
