use egui::{Id, Ui};
use std::any::Any;

#[macro_export]
macro_rules! mem_temp {
    ($ui:expr, $id:expr, $data:expr) => {
        $ui.memory_mut(|mem| mem.data.insert_temp($id, $data));
    };
    ($ui:expr, $id:expr) => {
        $ui.memory(|mem| mem.data.get_temp($id))
    };
}

#[macro_export]
macro_rules! mem_clear {
    ($ui:expr, $id:expr, $T:ty) => {
        $ui.memory_mut(|mem| mem.data.remove::<$T>($id))
    };
}

pub fn with_temp<T: 'static + Any + Clone + Send + Sync>(
    ui: &mut Ui,
    mem_id: Id,
    show: impl FnOnce(&mut Ui, Option<T>) -> Option<T>,
) {
    let data = ui.memory(|mem| mem.data.get_temp(mem_id));
    let new_data = show(ui, data);
    match new_data {
        None => ui.memory_mut(|mem| mem.data.remove::<T>(mem_id)),
        Some(data) => ui.memory_mut(|mem| mem.data.insert_temp(mem_id, data)),
    }
}
