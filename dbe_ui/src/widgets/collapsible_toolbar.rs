use crate::widgets::dpanel::{DPanel, DPanelSide};
use crate::widgets::rotated_label::{RotLabelDirection, SelectableRotLabel};
use egui::util::id_type_map::SerializableAny;
use egui::{
    vec2, Align, CentralPanel, Context, Frame, Id, Layout, Response, Rounding, Stroke, Style, Ui,
    Widget,
};
use inline_tweak::tweak;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::borrow::Cow;
use std::hash::Hash;
use std::sync::Arc;
use tracing::debug;

/// Trait for a tab that can be shown in the toolbar
pub trait ToolbarViewer {
    type Tab;

    /// Get the user-friendly name of the tab
    fn title(&self, tab: &Self::Tab) -> Cow<'_, str>;

    /// Whether the tab can be closed
    fn closable(&self, tab: &Self::Tab) -> bool;

    /// Show the header of the tab
    fn header(
        &mut self,
        ui: &mut Ui,
        tab: &Self::Tab,
        selected: bool,
        direction: RotLabelDirection,
    ) -> Response {
        SelectableRotLabel::new(selected, self.title(tab), direction).ui(ui)
    }

    /// Shows the content of the tab
    fn ui(&mut self, ui: &mut Ui, tab: &Self::Tab, direction: RotLabelDirection);
}

#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct CollapsibleToolbar<'a, Tab: SerializableAny> {
    side: DPanelSide,
    default_tabs_start: &'a [Tab],
    default_tabs_end: &'a [Tab],

    default_selected_start: Option<usize>,
    default_selected_end: Option<usize>,

    /// Frame used when the toolbar is expanded
    ///
    /// Defaults to [Frame::none()]
    expanded_frame: Option<Frame>,

    /// Frame used for the content of the toolbar
    ///
    /// Defaults to [Frame::central_panel()]
    content_frame: Option<Frame>,

    /// Frame used for the tabs of the toolbar
    ///
    /// Defaults to a frame with panel color and no rounding
    tabs_frame: Option<Frame>,

    /// Space at the start and end of the tabs panel
    tabs_margins: Option<f32>,

    button_style: Box<dyn Fn(&mut Style)>,

    persist: bool,

    global_drag_id: Option<Id>,
}

impl<Tab: SerializableAny> CollapsibleToolbar<'_, Tab> {}

impl<'a, Tab: SerializableAny> CollapsibleToolbar<'a, Tab> {
    pub fn new(
        side: DPanelSide,
        default_tabs_start: &'a [Tab],
        default_tabs_end: &'a [Tab],
    ) -> Self {
        Self {
            side,
            default_tabs_start,
            default_tabs_end,
            default_selected_start: None,
            default_selected_end: None,
            expanded_frame: None,
            content_frame: None,
            tabs_frame: None,
            tabs_margins: None,
            button_style: Box::new(default_button_style),
            persist: false,
            global_drag_id: None,
        }
    }

    /// Specifies whether the state of the toolbar should be persisted between sessions
    pub fn persist(mut self, persist: bool) -> Self {
        self.persist = persist;
        self
    }

    /// Specifies the index of the tab that should be selected by default in the start panel
    ///
    /// # Panics
    /// Will panic if the index is out of bounds
    pub fn default_selected_start(mut self, default_selected_start: usize) -> Self {
        self.default_selected_start = Some(default_selected_start);
        self
    }

    /// Specifies the index of the tab that should be selected by default in the end panel
    ///
    /// # Panics
    /// Will panic if the index is out of bounds
    pub fn default_selected_end(mut self, default_selected_end: usize) -> Self {
        self.default_selected_end = Some(default_selected_end);
        self
    }

    pub fn expanded_frame(mut self, expanded_frame: Frame) -> Self {
        self.expanded_frame = Some(expanded_frame);
        self
    }

    pub fn content_frame(mut self, content_frame: Frame) -> Self {
        self.content_frame = Some(content_frame);
        self
    }

    pub fn tabs_frame(mut self, tabs_frame: Frame) -> Self {
        self.tabs_frame = Some(tabs_frame);
        self
    }

    pub fn tabs_margins(mut self, tabs_margins: f32) -> Self {
        self.tabs_margins = Some(tabs_margins);
        self
    }

    pub fn button_style(mut self, button_style: Box<dyn Fn(&mut Style)>) -> Self {
        self.button_style = button_style;
        self
    }

    pub fn global_drag_id(mut self, global_drag_id: Id) -> Self {
        self.global_drag_id = Some(global_drag_id);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TabsInfo<Tab: 'static + Any + Clone + Send + Sync> {
    start: Arc<Vec<Tab>>,
    end: Arc<Vec<Tab>>,
    selected_start: Option<usize>,
    selected_end: Option<usize>,
}

impl<Tab: SerializableAny> TabsInfo<Tab> {
    pub fn new(
        start: impl Into<Arc<Vec<Tab>>>,
        end: impl Into<Arc<Vec<Tab>>>,
        selected_start: Option<usize>,
        selected_end: Option<usize>,
    ) -> Self {
        Self {
            start: start.into(),
            end: end.into(),
            selected_start,
            selected_end,
        }
    }
}

impl<Tab: SerializableAny + Eq + Hash> CollapsibleToolbar<'_, Tab> {
    pub fn show_inside(self, ui: &mut Ui, viewer: &mut impl ToolbarViewer<Tab = Tab>) -> Response {
        let tabs_id = ui.id().with("collapsible_toolbar");
        let state_id = tabs_id.with("state");

        let mut info = self.load_tabs_info(ui.ctx(), state_id);
        let is_expanded = info.selected_start.is_some() || info.selected_end.is_some();
        let res = DPanel::show_animated_between_inside(
            ui,
            is_expanded,
            self.tabs_panel(tabs_id, self.side, ui.ctx()),
            DPanel::new(ui.id(), self.side).frame(self.expanded_frame.unwrap_or_else(Frame::none)),
            |ui, how_expanded| {
                self.show_expanding(ui, how_expanded, self.side, tabs_id, &mut info, viewer)
            },
        )
        .inner;

        if res.changed() {
            self.save_tabs_info(ui.ctx(), state_id, info);
        };

        res
    }

    pub fn show(
        self,
        ctx: &Context,
        id: impl Into<Id>,
        viewer: &mut impl ToolbarViewer<Tab = Tab>,
    ) -> Option<Response> {
        let id = id.into();
        let tabs_id = id.with("collapsible_toolbar");
        let state_id = tabs_id.with("state");

        let mut info = self.load_tabs_info(ctx, state_id);
        let is_expanded = info.selected_start.is_some() || info.selected_end.is_some();

        let res = DPanel::show_animated_between(
            ctx,
            is_expanded,
            self.tabs_panel(tabs_id, self.side, ctx),
            DPanel::new(id, self.side)
                .frame(self.expanded_frame.unwrap_or_else(Frame::none))
                .min_size(tweak!(100.0))
                .resizable(true),
            |ui, how_expanded| {
                self.show_expanding(ui, how_expanded, self.side, tabs_id, &mut info, viewer)
            },
        )?;

        if res.inner.changed() {
            self.save_tabs_info(ctx, state_id, info);
        };

        Some(res.inner)
    }

    fn show_expanding(
        &self,
        ui: &mut Ui,
        how_expanded: f32,
        side: DPanelSide,
        tabs_id: Id,
        info: &mut TabsInfo<Tab>,
        viewer: &mut impl ToolbarViewer<Tab = Tab>,
    ) -> Response {
        if how_expanded > 0.0 {
            self.show_inside_inner(ui, side, tabs_id, info, viewer)
        } else {
            self.show_tabs(ui, side, tabs_id, info, viewer)
        }
    }

    fn show_inside_inner(
        &self,
        ui: &mut Ui,
        side: DPanelSide,
        tabs_id: Id,
        info: &mut TabsInfo<Tab>,
        viewer: &mut impl ToolbarViewer<Tab = Tab>,
    ) -> Response {
        let content_panel_id = tabs_id.with("content");
        ui.set_min_size(ui.available_size());

        let tabs_res = self
            .tabs_panel(tabs_id, side, ui.ctx())
            .show_inside(ui, |ui| self.show_tabs(ui, side, tabs_id, info, viewer))
            .inner;

        CentralPanel::default()
            .frame(
                self.content_frame
                    .unwrap_or_else(|| Frame::central_panel(ui.style())),
            )
            .show_inside(ui, |ui| match (info.selected_start, info.selected_end) {
                (Some(start), None) => viewer.ui(ui, &info.start[start], side.into()),
                (None, Some(end)) => viewer.ui(ui, &info.end[end], side.into()),
                (Some(start), Some(end)) => {
                    let (_, side_end) = side.perpendicular();
                    let available_size = side.available_size(ui);
                    let min_size = 100.0;
                    DPanel::new(content_panel_id.with("end"), side_end)
                        .frame(Frame::none())
                        .show_separator_line(true)
                        .resizable(true)
                        .min_size(min_size)
                        .max_size(available_size - min_size)
                        .show_animated_inside(ui, info.selected_end.is_some(), |ui| {
                            viewer.ui(ui, &info.end[end], side.into());
                        });

                    CentralPanel::default()
                        .frame(Frame::none())
                        .show_inside(ui, |ui| {
                            viewer.ui(ui, &info.start[start], side.into());
                        });
                }
                (None, None) => {}
            });

        tabs_res
    }

    fn show_tabs(
        &self,
        ui: &mut Ui,
        side: DPanelSide,
        tabs_panel_id: Id,
        info: &mut TabsInfo<Tab>,
        viewer: &mut impl ToolbarViewer<Tab = Tab>,
    ) -> Response {
        const MIN_SIZE: f32 = 100.0;

        #[allow(clippy::too_many_arguments)]
        fn layout_tabs<Tab: SerializableAny>(
            ui: &mut Ui,
            tabs_panel_id: Id,
            global_drag_id: Option<Id>,
            viewer: &mut impl ToolbarViewer<Tab = Tab>,
            info: &mut TabsInfo<Tab>,
            is_start: bool,
            direction: RotLabelDirection,
            layout: Layout,
        ) -> Response {
            let (tabs, selected) = if is_start {
                (&mut info.start, &mut info.selected_start)
            } else {
                (&mut info.end, &mut info.selected_end)
            };

            let drag_id = global_drag_id.unwrap_or_else(|| tabs_panel_id.with("drag"));

            let mut drag_state = DragInfo::<Tab>::load(ui.ctx(), drag_id);

            let timer_id = tabs_panel_id.with("timer");
            let time = ui.input(|input| input.time);

            let was_rearranging = drag_state.rearranging;

            let mut max_size = 0.0f32;
            let mut changed = false;
            let mut to_delete = None::<usize>;
            let mut to_insert = None::<(usize, Tab)>;

            let mut tabs_cb = |ui: &mut Ui| {
                if direction.is_horizontal() {
                    ui.set_min_width(MIN_SIZE);
                } else {
                    ui.set_min_height(MIN_SIZE);
                }

                ui.with_layout(layout, |ui| {
                    if drag_state.rearranging
                        && SelectableRotLabel::new(false, "✓", direction)
                            .ui(ui)
                            .clicked()
                    {
                        debug!("clicked finish button, stopping rearranging");
                        drag_state.rearranging = false;
                        changed = true;
                        ui.memory_mut(|mem| {
                            mem.data.remove_temp::<f64>(timer_id);
                        });
                    }
                    for (i, tab) in tabs.iter().enumerate() {
                        if drag_state.rearranging {
                            let res = ui.dnd_drag_source(
                                tabs_panel_id.with(is_start).with(i),
                                DragPayload::<Tab> {
                                    tab: tabs[i].clone(),
                                    panel: tabs_panel_id,
                                    is_start,
                                    index: i,
                                    drag_id,
                                },
                                |ui| viewer.header(ui, tab, false, direction),
                            );
                            if let (Some(pointer), Some(_)) = (
                                ui.input(|i| i.pointer.interact_pos()),
                                res.response.dnd_hover_payload::<DragPayload<Tab>>(),
                            ) {
                                let rect = res.response.rect;

                                let stroke = Stroke::new(1.0, egui::Color32::WHITE);

                                let (before, range, start, end) = if direction.is_horizontal() {
                                    (
                                        pointer.x < rect.center().x,
                                        rect.y_range(),
                                        rect.left(),
                                        rect.right(),
                                    )
                                } else {
                                    (
                                        pointer.y < rect.center().y,
                                        rect.x_range(),
                                        rect.top(),
                                        rect.bottom(),
                                    )
                                };
                                let insert_row_idx = if before {
                                    // Above us
                                    if direction.is_vertical() {
                                        ui.painter().hline(range, start, stroke);
                                    } else {
                                        ui.painter().vline(start, range, stroke);
                                    }
                                    if is_start {
                                        i
                                    } else {
                                        i + 1
                                    }
                                } else {
                                    // Below us
                                    if direction.is_vertical() {
                                        ui.painter().hline(range, end, stroke);
                                    } else {
                                        ui.painter().vline(end, range, stroke);
                                    }
                                    if is_start {
                                        i + 1
                                    } else {
                                        i
                                    }
                                };
                                if let Some(dragged_payload) =
                                    res.response.dnd_release_payload::<DragPayload<Tab>>()
                                {
                                    debug!("tab dropped, inserting at position {}", insert_row_idx);
                                    to_insert = Some((insert_row_idx, dragged_payload.tab.clone()));
                                    drag_state.last_drop = Some(dragged_payload);
                                }
                            }
                            if let Some(dropped_payload) = &drag_state.last_drop {
                                if dropped_payload.panel == tabs_panel_id
                                    && dropped_payload.is_start == is_start
                                    && dropped_payload.index == i
                                {
                                    to_delete = Some(i);
                                    drag_state.last_drop = None;
                                }
                            }
                            continue;
                        }

                        let is_selected = *selected == Some(i);
                        let header = viewer.header(ui, tab, is_selected, direction);
                        if header.clicked() {
                            if is_selected {
                                *selected = None;
                            } else {
                                *selected = Some(i);
                            }
                            changed = true;
                        }

                        max_size = max_size.max(if direction.is_horizontal() {
                            header.rect.height()
                        } else {
                            header.rect.width()
                        });

                        header.context_menu(|ui| {
                            if viewer.closable(tab) && ui.button("Close").clicked() {
                                to_delete = Some(i);
                                ui.close_menu();
                            }
                            if ui.button("Rearrange").clicked() {
                                debug!("started rearranging");
                                drag_state.rearranging = true;
                                ui.memory_mut(|mem| {
                                    *mem.data.get_temp_mut_or_insert_with(timer_id, || time) = time
                                });
                                changed = true;
                                ui.close_menu();
                            }
                        });
                    }
                })
            };

            let res = if was_rearranging {
                let (res, payload) =
                    ui.dnd_drop_zone::<DragPayload<Tab>, _>(Frame::none(), &mut tabs_cb);
                if let Some(payload) = payload {
                    if payload.drag_id == drag_id {
                        to_insert = Some((tabs.len(), payload.tab.clone()));
                        drag_state.last_drop = Some(payload);
                        debug!("dropped payload, inserting at end");
                    }
                }

                res.inner
            } else {
                tabs_cb(ui)
            };

            let mut res = res.response;
            if let Some(to_delete) = to_delete {
                Arc::make_mut(tabs).remove(to_delete);
                if *selected == Some(to_delete) {
                    if to_delete > 0 {
                        *selected = Some(to_delete - 1);
                    } else {
                        *selected = None;
                    }
                }
                changed = true;

                if let Some(to_insert) = &mut to_insert {
                    if to_delete < to_insert.0 {
                        to_insert.0 -= 1;
                    }
                }
            }

            if let Some((insert_row_idx, tab)) = to_insert {
                let tabs_mut = Arc::make_mut(tabs);
                if insert_row_idx == tabs_mut.len() {
                    tabs_mut.push(tab);
                } else {
                    tabs_mut.insert(insert_row_idx, tab);
                }
                changed = true;
            }

            if was_rearranging && drag_state.rearranging && res.clicked_elsewhere() {
                let start_time = ui.memory(|mem| mem.data.get_temp::<f64>(timer_id));
                if let Some(start_time) = start_time {
                    let elapsed = time - start_time;
                    if elapsed > 0.5 {
                        debug!("clicked elsewhere, stopping rearranging");
                        drag_state.rearranging = false;
                        changed = true;
                        ui.memory_mut(|mem| {
                            mem.data.remove_temp::<f64>(timer_id);
                        });
                    }
                }
            }
            if changed {
                res.mark_changed()
            }

            drag_state.save(ui.ctx(), drag_id);
            record_tabs_size(ui.ctx(), tabs_panel_id, max_size);
            res
        }

        let (side_start, _) = side.perpendicular();
        let (start_layout, end_layout) = match side {
            DPanelSide::Left | DPanelSide::Right => (
                Layout::top_down(Align::Center),
                Layout::bottom_up(Align::Center),
            ),
            DPanelSide::Top | DPanelSide::Bottom => (
                Layout::left_to_right(Align::Center),
                Layout::right_to_left(Align::Center),
            ),
        };

        let res_start = DPanel::new(tabs_panel_id.with("start"), side_start)
            .resizable(false)
            .frame(Frame::none())
            .show_separator_line(false)
            .show_inside(ui, |ui| {
                (self.button_style)(ui.style_mut());
                layout_tabs(
                    ui,
                    tabs_panel_id,
                    self.global_drag_id,
                    viewer,
                    info,
                    true,
                    side.into(),
                    start_layout,
                )
            })
            .inner;

        let res_end = CentralPanel::default()
            .frame(Frame::none())
            .show_inside(ui, |ui| {
                (self.button_style)(ui.style_mut());
                layout_tabs(
                    ui,
                    tabs_panel_id,
                    self.global_drag_id,
                    viewer,
                    info,
                    false,
                    side.into(),
                    end_layout,
                )
            })
            .inner;

        res_start | res_end
    }

    fn tabs_panel(&self, tabs_panel_id: Id, side: DPanelSide, ctx: &Context) -> DPanel {
        let style = ctx.style();
        let mut frame = self.tabs_frame.unwrap_or_else(|| {
            Frame::none()
                .inner_margin(tweak!(1.0))
                .fill(style.visuals.panel_fill)
        });

        let margins = self.tabs_margins.unwrap_or(0.0);

        if margins != 0.0 {
            if side.is_top_bottom() {
                frame.inner_margin.left = margins;
                frame.inner_margin.right = margins;
            } else {
                frame.inner_margin.top = margins;
                frame.inner_margin.bottom = margins;
            }
        }

        // let line_height = get_tabs_height(ctx, tabs_panel_id).unwrap_or_else(|| {
        //     dbg!(ctx.fonts(|f| f.row_height(&TextStyle::Button.resolve(&style))))
        //         + style.spacing.button_padding.y * 2.0
        // });

        let line_height = get_tabs_size(ctx, tabs_panel_id).unwrap_or(0.0);
        let panel = DPanel::new(tabs_panel_id, side)
            .resizable(false)
            .show_separator_line(false)
            .frame(frame);

        if line_height != 0.0 {
            let margins = frame.inner_margin.sum();
            panel.exact_size(
                line_height
                    + if side.is_top_bottom() {
                        margins.y
                    } else {
                        margins.x
                    },
            )
        } else {
            panel
        }
    }

    fn load_tabs_info(&self, ctx: &Context, state_id: Id) -> TabsInfo<Tab> {
        ctx.memory_mut(|mem| {
            if self.persist {
                mem.data.get_persisted_mut_or_insert_with(state_id, || {
                    TabsInfo::new(
                        self.default_tabs_start.to_vec(),
                        self.default_tabs_end.to_vec(),
                        self.default_selected_start,
                        self.default_selected_end,
                    )
                })
            } else {
                mem.data.get_temp_mut_or_insert_with(state_id, || {
                    TabsInfo::new(
                        self.default_tabs_start.to_vec(),
                        self.default_tabs_end.to_vec(),
                        self.default_selected_start,
                        self.default_selected_end,
                    )
                })
            }
            .clone()
        })
    }

    fn save_tabs_info(&self, ctx: &Context, state_id: Id, info: TabsInfo<Tab>) {
        ctx.memory_mut(|mem| {
            if self.persist {
                mem.data.insert_persisted(state_id, info);
            } else {
                mem.data.insert_temp(state_id, info);
            }
        });
    }
}

#[derive(Debug, Clone)]
struct DragPayload<Tab> {
    tab: Tab,
    panel: Id,
    is_start: bool,
    index: usize,
    drag_id: Id,
}

#[derive(Debug, Clone)]
struct DragInfo<Tab> {
    rearranging: bool,
    last_drop: Option<Arc<DragPayload<Tab>>>,
}

impl<Tab: Any + Clone + Send + Sync> DragInfo<Tab> {
    pub fn load(ctx: &Context, id: Id) -> Self {
        ctx.memory_mut(|mem| {
            mem.data
                .get_temp_mut_or_insert_with(id, || Self {
                    rearranging: false,
                    last_drop: None,
                })
                .clone()
        })
    }

    pub fn save(&self, ctx: &Context, id: Id) {
        ctx.memory_mut(|mem| {
            mem.data.insert_temp(id, self.clone());
        });
    }
}

fn default_button_style(style: &mut Style) {
    style.visuals.widgets.active.rounding = Rounding::ZERO;
    style.visuals.widgets.inactive.rounding = Rounding::ZERO;
    style.visuals.widgets.hovered.rounding = Rounding::ZERO;
    style.visuals.widgets.active.bg_stroke = Stroke::NONE;
    style.visuals.widgets.inactive.bg_stroke = Stroke::NONE;
    style.visuals.widgets.hovered.bg_stroke = Stroke::NONE;
    style.visuals.selection.bg_fill = style.visuals.extreme_bg_color;
    style.visuals.selection.stroke = style.visuals.widgets.active.fg_stroke;
    style.spacing.button_padding = vec2(tweak!(6.0), tweak!(4.0));
}

fn record_tabs_size(ctx: &Context, tabs_id: Id, size: f32) {
    if size <= 1.0 {
        return;
    }

    ctx.memory_mut(|mem| {
        mem.data
            .insert_temp(tabs_id.with("tabs_height_tracker"), size);
    });
}

fn get_tabs_size(ctx: &Context, tabs_id: Id) -> Option<f32> {
    ctx.memory(|mem| mem.data.get_temp(tabs_id.with("tabs_height_tracker")))
}
