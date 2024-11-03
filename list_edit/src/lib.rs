use egui::{Id, Rect, Sense, Separator, Ui, Vec2, Widget};
pub use egui_dnd::ItemState;
use egui_dnd::{dnd, DragDropItem};
use inline_tweak::tweak;
use std::hash::Hash;
use std::marker::PhantomData;

pub struct ListEditor<T, NewItem, CanDelete, IdSource> {
    new_item: NewItem,
    can_delete: CanDelete,
    show_new_button: bool,
    id: IdSource,
    data: PhantomData<T>,
}

struct DragWrapper<T>(Id, T);
impl<T> DragDropItem for DragWrapper<T> {
    fn id(&self) -> Id {
        self.0
    }
}

impl<T, NewItem: Fn(usize) -> T, CanDelete: Fn(usize, T) -> bool, IdSource: Hash>
    ListEditor<T, NewItem, CanDelete, IdSource>
{
    pub fn show(
        self,
        ui: &mut Ui,
        items: &mut Vec<T>,
        mut display: impl FnMut(&mut Ui, ItemState, &mut T),
    ) {
        let id = self.id.id();

        ui.vertical(|ui| {
            let mut delete_id = None;
            let mut last_item_width = 0.0;
            let response = dnd(ui, id).with_return_animation_time(0.0).show(
                items
                    .iter_mut()
                    .enumerate()
                    .map(|(i, e)| DragWrapper(id.with(i), e)),
                |ui, item: DragWrapper<&mut T>, handle, state| {
                    let res = ui.horizontal_top(|ui| {
                        ui.push_id(state.index, |ui| {
                            let id = id.with(state.index).with("_handle_sizer");
                            let last_item_height: Option<f32> =
                                ui.memory_mut(|mem| mem.data.get_temp(id));

                            let handle_content = |ui: &mut Ui| {
                                let (res_a, res_b) = ui
                                    .push_id("_handle_sizer", |ui| {
                                        let style = ui.style_mut();
                                        style.visuals.widgets.noninteractive.bg_stroke.color =
                                            style.visuals.widgets.active.fg_stroke.color;
                                        fn separator() -> Separator {
                                            Separator::default()
                                                .spacing(tweak!(1.0))
                                                .shrink(tweak!(2.0))
                                        };
                                        let res_left = separator().ui(ui);
                                        separator().ui(ui);
                                        let res_right = separator().ui(ui);
                                        (res_left, res_right)
                                    })
                                    .inner;

                                let rect = Rect::from_two_pos(
                                    res_a.rect.left_top(),
                                    res_b.rect.right_bottom(),
                                );

                                let res = ui.interact(
                                    rect,
                                    id.with(state.index).with("_sensor"),
                                    Sense::click(),
                                );
                                res.context_menu(|ui| {
                                    if ui.button("Delete").clicked() {
                                        delete_id = Some(state.index);
                                        ui.close_menu();
                                    }
                                });
                            };

                            match last_item_height {
                                None => handle.ui(ui, handle_content),
                                Some(h) => handle.ui_sized(ui, Vec2::new(24.0, h), handle_content),
                            };

                            let item_res = ui.horizontal(|ui| display(ui, state, item.1));
                            let item_height = item_res.response.rect.size().y;
                            ui.memory_mut(|mem| mem.data.insert_temp(id, item_height));
                            match last_item_height {
                                None => ui.ctx().request_repaint(),
                                Some(last_height) if last_height != item_height => {
                                    ui.ctx().request_repaint()
                                }
                                _ => {}
                            };
                        });
                    });
                    last_item_width = res.response.rect.size().x;
                },
            );

            if response.is_drag_finished() {
                response.update_vec(items);
            }

            if let Some(id) = delete_id {
                items.remove(id);
            }

            let add_button = egui::Button::new("➕").min_size(Vec2::new(last_item_width, 0.0));

            if ui.add(add_button).clicked() {
                items.push((self.new_item)(items.len()));
            }
        });
    }
}

impl<T> Default for ListEditor<T, (), (), ()> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ListEditor<T, (), (), ()> {
    pub fn new() -> Self {
        Self {
            new_item: (),
            can_delete: (),
            show_new_button: true,
            id: (),
            data: Default::default(),
        }
    }
}

impl<T, NewItem, CanDelete, Id> ListEditor<T, NewItem, CanDelete, Id> {
    pub fn can_delete<NewCanDelete: Fn(usize, T) -> bool>(
        self,
        can_delete: NewCanDelete,
    ) -> ListEditor<T, NewItem, NewCanDelete, Id> {
        ListEditor {
            can_delete,
            show_new_button: self.show_new_button,
            new_item: self.new_item,
            id: self.id,
            data: Default::default(),
        }
    }

    pub fn new_item<NewNewItem: Fn(usize) -> T>(
        self,
        new_item: NewNewItem,
    ) -> ListEditor<T, NewNewItem, CanDelete, Id> {
        ListEditor {
            can_delete: self.can_delete,
            show_new_button: self.show_new_button,
            new_item,
            id: self.id,
            data: Default::default(),
        }
    }

    pub fn id_source<NewId: Hash>(
        self,
        id_source: NewId,
    ) -> ListEditor<T, NewItem, CanDelete, NewId> {
        ListEditor {
            can_delete: self.can_delete,
            show_new_button: self.show_new_button,
            new_item: self.new_item,
            id: id_source,
            data: Default::default(),
        }
    }
}

pub fn list_editor<T, IdSource: Hash>(
    id: IdSource,
) -> ListEditor<T, (), impl Fn(usize, T) -> bool, IdSource> {
    ListEditor::new().id_source(id).can_delete(|_, _| true)
}
