use crate::error::report_error;
use crate::DbeApp;
use egui::Ui;
use egui_colors::tokens::ThemeColor;
use egui_colors::{Colorix, Theme};
use egui_hooks::UseHookExt;
use miette::miette;
use std::ops::DerefMut;

pub fn colors_tab(ui: &mut Ui, app: &mut DbeApp, global: bool) {
    colorix_editor(ui, &mut app.colorix, global);
}

pub fn colorix_editor(ui: &mut Ui, colorix: &mut Colorix, global: bool) {
    if global {
        colorix.light_dark_toggle_button(ui, 32.0);
    }
    colorix.ui_combo_12(ui, false);

    ui.separator();

    change_all_combo(ui, colorix, global);

    ui.separator();

    import_export(ui, colorix, global);
}

fn change_all_combo(ui: &mut Ui, colorix: &mut Colorix, global: bool) {
    let dropdown_colors: [ThemeColor; 23] = [
        ThemeColor::Gray,
        ThemeColor::EguiBlue,
        ThemeColor::Tomato,
        ThemeColor::Red,
        ThemeColor::Ruby,
        ThemeColor::Crimson,
        ThemeColor::Pink,
        ThemeColor::Plum,
        ThemeColor::Purple,
        ThemeColor::Violet,
        ThemeColor::Iris,
        ThemeColor::Indigo,
        ThemeColor::Blue,
        ThemeColor::Cyan,
        ThemeColor::Teal,
        ThemeColor::Jade,
        ThemeColor::Green,
        ThemeColor::Grass,
        ThemeColor::Brown,
        ThemeColor::Bronze,
        ThemeColor::Gold,
        ThemeColor::Orange,
        ThemeColor::Custom([0, 0, 0]),
    ];

    let mut color = ui.use_state(|| ThemeColor::Gray, ()).into_var();
    let mut change_all = false;

    let color_edit_size = egui::vec2(40.0, 18.0);

    ui.horizontal(|ui| {
        if let ThemeColor::Custom(rgb) = color.deref_mut() {
            let re = ui.color_edit_button_srgb(rgb);
            if re.changed() {
                change_all = true;
            }
        } else {
            // Allocate a color edit button's worth of space for non-custom presets,
            // for alignment purposes.
            ui.add_space(color_edit_size.x + ui.style().spacing.item_spacing.x);
        }

        ui.add_space(color_edit_size.x + ui.style().spacing.item_spacing.x);

        // egui::widgets::color_picker::show_color(ui, color.rgb(), color_edit_size);
        egui::ComboBox::from_label("Change all colors")
            .selected_text(color.label())
            .show_ui(ui, |ui| {
                for preset in dropdown_colors {
                    if ui
                        .selectable_value(color.deref_mut(), preset, preset.label())
                        .clicked()
                    {
                        change_all = true;
                    };
                }
            });

        if change_all {
            if global {
                colorix.update_theme(ui.ctx(), [*color; 12])
            } else {
                *colorix = Colorix::local_from_style([*color; 12], colorix.dark_mode())
            }
        }
    });

    ui.horizontal(|ui| {
        ui.add_space((color_edit_size.x + ui.style().spacing.item_spacing.x) * 2.0);
        colorix.themes_dropdown(ui, None, false);
        ui.label("Preset theme")
    });
}

fn import_export(ui: &mut Ui, colorix: &mut Colorix, global: bool) {
    let mut text = ui.use_state(|| "".to_string(), ()).into_var();
    ui.horizontal(|ui| {
        if ui.button("Export").clicked() {
            *text = serde_json5::to_string(colorix.theme()).unwrap();
        }
        if ui
            .add_enabled(!text.trim().is_empty(), egui::Button::new("Import"))
            .clicked()
        {
            match serde_json5::from_str::<Theme>(&text) {
                Ok(theme) => {
                    if global {
                        colorix.update_theme(ui.ctx(), theme)
                    } else {
                        *colorix = Colorix::local_from_style(theme, colorix.dark_mode())
                    }
                }
                Err(e) => {
                    report_error(miette!("Failed to import theme: {}", e));
                }
            }
        }
    });
    ui.text_edit_multiline(text.deref_mut());
}
