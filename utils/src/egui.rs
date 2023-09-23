#[macro_export]
macro_rules! mem_temp {
    ($ui:expr, $id:expr, $data:expr) => {
        $ui.memory_mut(|mem| mem.data.insert_temp($id, $data));
    };
    ($ui:expr, $id:expr) => {
        $ui.memory(|mem| mem.data.get_temp($id))
    };
}
