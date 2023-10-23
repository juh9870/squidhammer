use crate::states::main_state::TabHandler;
use crate::value::etype::registry::eenum::EEnumData;
use crate::value::etype::registry::eitem::EItemType;
use crate::value::etype::registry::estruct::EStructData;
use crate::value::etype::registry::EObjectType;
use egui::Ui;
use utils::mem_temp;

pub(super) fn show_types_debugger(state: &TabHandler, ui: &mut Ui) {
    let search_id = ui.id().with("search_query");
    let mut search: String = mem_temp!(ui, search_id).unwrap_or_default();
    ui.vertical(|ui| {
        ui.text_edit_singleline(&mut search);
        for obj in state
            .0
            .state
            .registry
            .borrow()
            .all_objects_filtered(&search)
        {
            ui.push_id(obj.id().to_string(), |ui| {
                ui.group(|ui| match obj {
                    EObjectType::Struct(s) => {
                        show_struct(ui, s);
                    }
                    EObjectType::Enum(e) => {
                        show_enum(ui, e);
                    }
                });
            });
        }
    });
    mem_temp!(ui, search_id, search);
}

fn show_struct(ui: &mut Ui, s: &EStructData) {
    ui.vertical(|ui| {
        ui.heading(s.ident.to_string());
        if !s.generic_arguments.is_empty() {
            ui.horizontal(|ui| {
                ui.label("Generics: ");
                for name in &s.generic_arguments {
                    ui.monospace(name.as_str());
                }
            });
        }
        egui::Grid::new(ui.id().with("field"))
            .num_columns(3)
            .show(ui, |ui| {
                for field in &s.fields {
                    ui.label(field.name.as_str());
                    show_item(ui, &field.ty);
                    ui.end_row();
                }
            });
    });
}

fn show_enum(ui: &mut Ui, e: &EEnumData) {
    ui.vertical(|ui| {
        ui.heading(e.ident.to_string());
        if !e.generic_arguments.is_empty() {
            ui.horizontal(|ui| {
                ui.label("Generics: ");
                for name in &e.generic_arguments {
                    ui.monospace(name.as_str());
                }
            });
        }
        egui::Grid::new(ui.id().with("field"))
            .num_columns(4)
            .show(ui, |ui| {
                for variant in &e.variants {
                    ui.label(&variant.name);
                    show_item(ui, &variant.data);
                    ui.label(variant.pat.to_string());
                    // ui.label(field.name.as_str());
                    // show_item(ui, &field.ty);
                    ui.end_row();
                }
            });
    });
}

fn show_item(ui: &mut Ui, ty: &EItemType) {
    ui.label(ty.as_ref());
    ui.label(format!("{:?}", ty));
}
