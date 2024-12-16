use crate::widgets::dpanel::DPanelSide;
use egui::{
    epaint, vec2, NumExt, Response, SelectableLabel, Sense, TextStyle, TextWrapMode, Ui, Vec2,
    Widget, WidgetInfo, WidgetText, WidgetType,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum RotLabelDirection {
    Horizontal,
    VerticalLeft,
    VerticalRight,
}

impl From<DPanelSide> for RotLabelDirection {
    fn from(side: DPanelSide) -> Self {
        match side {
            DPanelSide::Left => RotLabelDirection::VerticalLeft,
            DPanelSide::Right => RotLabelDirection::VerticalRight,
            DPanelSide::Top | DPanelSide::Bottom => RotLabelDirection::Horizontal,
        }
    }
}

impl RotLabelDirection {
    pub fn angle(self) -> f32 {
        match self {
            RotLabelDirection::Horizontal => 0.0,
            RotLabelDirection::VerticalLeft => -std::f32::consts::FRAC_PI_2,
            RotLabelDirection::VerticalRight => std::f32::consts::FRAC_PI_2,
        }
    }

    pub fn is_horizontal(self) -> bool {
        self == RotLabelDirection::Horizontal
    }

    pub fn is_vertical(self) -> bool {
        !self.is_horizontal()
    }
}

#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct SelectableRotLabel {
    selected: bool,
    text: WidgetText,
    direction: RotLabelDirection,
}

impl SelectableRotLabel {
    pub fn new(selected: bool, text: impl Into<WidgetText>, direction: RotLabelDirection) -> Self {
        Self {
            selected,
            text: text.into(),
            direction,
        }
    }
}

impl Widget for SelectableRotLabel {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            selected,
            text,
            direction,
        } = self;

        if direction == RotLabelDirection::Horizontal {
            return SelectableLabel::new(selected, text).ui(ui);
        }

        #[inline(always)]
        fn transpose(vec: Vec2) -> Vec2 {
            vec2(vec.y, vec.x)
        }

        let button_padding = transpose(ui.spacing().button_padding);
        let total_extra = button_padding + button_padding;

        let wrap_height = ui.available_height() - total_extra.y;
        let galley = text.into_galley(
            ui,
            TextWrapMode::Extend.into(),
            wrap_height,
            TextStyle::Button,
        );

        let mut desired_size = total_extra + transpose(galley.size());
        desired_size.x = desired_size.x.at_least(ui.spacing().interact_size.y); // Use interact_size.y to match button height, even tho we are calculating width
        let (rect, response) = ui.allocate_at_least(desired_size, Sense::click());
        response.widget_info(|| {
            WidgetInfo::selected(
                WidgetType::SelectableLabel,
                ui.is_enabled(),
                selected,
                galley.text(),
            )
        });

        if ui.is_rect_visible(response.rect) {
            let visuals = ui.style().interact_selectable(&response, selected);

            if selected || response.hovered() || response.highlighted() || response.has_focus() {
                let rect = rect.expand(visuals.expansion);

                ui.painter().rect(
                    rect,
                    visuals.rounding,
                    visuals.weak_bg_fill,
                    visuals.bg_stroke,
                );
            }

            let pos = if direction == RotLabelDirection::VerticalRight {
                rect.right_top() + vec2(-button_padding.x, button_padding.y)
            } else {
                rect.left_bottom() + vec2(button_padding.x, -button_padding.y)
            };

            ui.painter().add(
                epaint::TextShape::new(pos, galley, visuals.text_color())
                    .with_angle(direction.angle()),
            );
        }

        response
    }
}
