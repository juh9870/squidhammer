use egui::{Context, Frame, InnerResponse, Rangef, TopBottomPanel};
use egui::{Id, SidePanel, Ui};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum DPanelSide {
    Left,
    Right,
    Top,
    Bottom,
}

impl DPanelSide {
    /// Returns the perpendicular sides to this side.
    ///
    /// Returned sides are always ordered left to right or top to bottom.
    pub fn perpendicular(self) -> (Self, Self) {
        match self {
            DPanelSide::Left | DPanelSide::Right => (DPanelSide::Top, DPanelSide::Bottom),
            DPanelSide::Top | DPanelSide::Bottom => (DPanelSide::Left, DPanelSide::Right),
        }
    }

    pub fn opposite(self) -> Self {
        match self {
            DPanelSide::Left => DPanelSide::Right,
            DPanelSide::Right => DPanelSide::Left,
            DPanelSide::Top => DPanelSide::Bottom,
            DPanelSide::Bottom => DPanelSide::Top,
        }
    }

    pub fn is_side(self) -> bool {
        match self {
            DPanelSide::Left | DPanelSide::Right => true,
            DPanelSide::Top | DPanelSide::Bottom => false,
        }
    }

    pub fn is_top_bottom(self) -> bool {
        !self.is_side()
    }

    pub fn available_size(self, ui: &Ui) -> f32 {
        if self.is_side() {
            ui.available_height()
        } else {
            ui.available_width()
        }
    }
}

#[must_use = "You should call .show()"]
pub enum DPanel {
    Side(SidePanel),
    TopBottom(TopBottomPanel),
}

impl DPanel {
    pub fn new(id: Id, direction: DPanelSide) -> Self {
        match direction {
            DPanelSide::Left => Self::Side(SidePanel::left(id)),
            DPanelSide::Right => Self::Side(SidePanel::right(id)),
            DPanelSide::Top => Self::TopBottom(TopBottomPanel::top(id)),
            DPanelSide::Bottom => Self::TopBottom(TopBottomPanel::bottom(id)),
        }
    }

    pub fn resizable(self, resizable: bool) -> Self {
        match self {
            DPanel::Side(panel) => DPanel::Side(panel.resizable(resizable)),
            DPanel::TopBottom(panel) => DPanel::TopBottom(panel.resizable(resizable)),
        }
    }

    pub fn show_separator_line(self, show: bool) -> Self {
        match self {
            DPanel::Side(panel) => DPanel::Side(panel.show_separator_line(show)),
            DPanel::TopBottom(panel) => DPanel::TopBottom(panel.show_separator_line(show)),
        }
    }

    pub fn frame(self, frame: Frame) -> Self {
        match self {
            DPanel::Side(panel) => DPanel::Side(panel.frame(frame)),
            DPanel::TopBottom(panel) => DPanel::TopBottom(panel.frame(frame)),
        }
    }

    pub fn default_size(self, size: f32) -> Self {
        match self {
            DPanel::Side(panel) => DPanel::Side(panel.default_width(size)),
            DPanel::TopBottom(panel) => DPanel::TopBottom(panel.default_height(size)),
        }
    }

    pub fn min_size(self, size: f32) -> Self {
        match self {
            DPanel::Side(panel) => DPanel::Side(panel.min_width(size)),
            DPanel::TopBottom(panel) => DPanel::TopBottom(panel.min_height(size)),
        }
    }

    pub fn max_size(self, size: f32) -> Self {
        match self {
            DPanel::Side(panel) => DPanel::Side(panel.max_width(size)),
            DPanel::TopBottom(panel) => DPanel::TopBottom(panel.max_height(size)),
        }
    }

    pub fn size_range(self, size_range: impl Into<Rangef>) -> Self {
        match self {
            DPanel::Side(panel) => DPanel::Side(panel.width_range(size_range)),
            DPanel::TopBottom(panel) => DPanel::TopBottom(panel.height_range(size_range)),
        }
    }

    pub fn exact_size(self, size: f32) -> Self {
        match self {
            DPanel::Side(panel) => DPanel::Side(panel.exact_width(size)),
            DPanel::TopBottom(panel) => DPanel::TopBottom(panel.exact_height(size)),
        }
    }

    pub fn is_side(&self) -> bool {
        match self {
            DPanel::Side(_) => true,
            DPanel::TopBottom(_) => false,
        }
    }

    pub fn is_top_bottom(&self) -> bool {
        !self.is_side()
    }
}

impl DPanel {
    pub fn show_inside<R>(
        self,
        ui: &mut Ui,
        content: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        match self {
            DPanel::Side(panel) => panel.show_inside(ui, content),
            DPanel::TopBottom(panel) => panel.show_inside(ui, content),
        }
    }

    pub fn show_animated_inside<R>(
        self,
        ui: &mut Ui,
        is_expanded: bool,
        content: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<R>> {
        match self {
            DPanel::Side(panel) => panel.show_animated_inside(ui, is_expanded, content),
            DPanel::TopBottom(panel) => panel.show_animated_inside(ui, is_expanded, content),
        }
    }

    pub fn show<R>(
        self,
        ctx: &Context,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        match self {
            DPanel::Side(panel) => panel.show(ctx, add_contents),
            DPanel::TopBottom(panel) => panel.show(ctx, add_contents),
        }
    }

    pub fn show_animated_between_inside<R>(
        ui: &mut Ui,
        is_expanded: bool,
        collapsed_panel: Self,
        expanded_panel: Self,
        add_contents: impl FnOnce(&mut Ui, f32) -> R,
    ) -> InnerResponse<R> {
        match (collapsed_panel, expanded_panel) {
            (DPanel::Side(collapsed), DPanel::Side(expanded)) => {
                SidePanel::show_animated_between_inside(
                    ui,
                    is_expanded,
                    collapsed,
                    expanded,
                    add_contents,
                )
            }
            (DPanel::TopBottom(collapsed), DPanel::TopBottom(expanded)) => {
                TopBottomPanel::show_animated_between_inside(
                    ui,
                    is_expanded,
                    collapsed,
                    expanded,
                    add_contents,
                )
            }
            _ => panic!("Cannot animate between different panel types"),
        }
    }

    pub fn show_animated<R>(
        self,
        ctx: &Context,
        is_expanded: bool,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<R>> {
        match self {
            DPanel::Side(panel) => panel.show_animated(ctx, is_expanded, add_contents),
            DPanel::TopBottom(panel) => panel.show_animated(ctx, is_expanded, add_contents),
        }
    }

    pub fn show_animated_between<R>(
        ctx: &Context,
        is_expanded: bool,
        collapsed_panel: Self,
        expanded_panel: Self,
        add_contents: impl FnOnce(&mut Ui, f32) -> R,
    ) -> Option<InnerResponse<R>> {
        match (collapsed_panel, expanded_panel) {
            (DPanel::Side(collapsed), DPanel::Side(expanded)) => SidePanel::show_animated_between(
                ctx,
                is_expanded,
                collapsed,
                expanded,
                add_contents,
            ),
            (DPanel::TopBottom(collapsed), DPanel::TopBottom(expanded)) => {
                TopBottomPanel::show_animated_between(
                    ctx,
                    is_expanded,
                    collapsed,
                    expanded,
                    add_contents,
                )
            }
            _ => panic!("Cannot animate between different panel types"),
        }
    }
}
