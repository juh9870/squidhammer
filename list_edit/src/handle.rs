use crate::{Sense, Ui, Vec2, Widget};
use egui::{pos2, vec2, Color32, Response, Stroke};
use inline_tweak::tweak;

/// A visual separator. A horizontal or vertical line (depending on [`crate::Layout`]).
///
/// Usually you'd use the shorter version [`Ui::separator`].
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// // These are equivalent:
/// ui.separator();
/// ui.add(egui::Separator::default());
/// # });
/// ```
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct Handle {
    spacing: Vec2,
    width: f32,
    dot_size: f32,
    margins: Vec2,
    color: Option<Color32>,
}

impl Default for Handle {
    fn default() -> Self {
        Self {
            spacing: Vec2::splat(1.0),
            width: 3.0,
            dot_size: 1.0,
            margins: Vec2::splat(0.0),
            color: None,
        }
    }
}

impl Handle {
    #[inline]
    pub fn spacing(mut self, spacing: Vec2) -> Self {
        self.spacing = spacing;
        self
    }

    #[inline]
    pub fn margins(mut self, margins: Vec2) -> Self {
        self.margins = margins;
        self
    }

    #[inline]
    pub fn dot_size(mut self, dot_size: f32) -> Self {
        self.dot_size = dot_size;
        self
    }

    #[inline]
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    #[inline]
    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }
}

impl Widget for Handle {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            spacing,
            width,
            margins,
            dot_size,
            color,
        } = self;

        let available_space = if ui.is_sizing_pass() {
            Vec2::ZERO
        } else {
            ui.available_size_before_wrap()
        };

        let size = vec2(width, available_space.y);

        let inner_size = size - margins * 2.0;
        let columns = ((inner_size.x + spacing.x) / (dot_size + spacing.x)).floor() as usize;
        let rows = ((inner_size.y + spacing.y) / (dot_size + spacing.y)).floor() as usize;

        let dots_area_size = vec2(
            columns as f32 * (dot_size + spacing.x) - spacing.x,
            rows as f32 * (dot_size + spacing.y) - spacing.y,
        );

        let actual_margin = (size - dots_area_size) / 2.0;

        let (rect, response) = ui.allocate_at_least(size, Sense::hover());

        if ui.is_rect_visible(response.rect) {
            let color =
                color.unwrap_or_else(|| ui.visuals().widgets.noninteractive.bg_stroke.color);
            let painter = ui.painter();
            for row in 0..rows {
                for column in 0..columns {
                    let x = rect.left() + actual_margin.x + column as f32 * (dot_size + spacing.x);
                    let y = rect.top() + actual_margin.y + row as f32 * (dot_size + spacing.y);
                    let rect = egui::Rect::from_min_size(pos2(x, y), vec2(dot_size, dot_size));
                    painter.rect(rect, tweak!(0.0), color, Stroke::NONE);
                }
            }
        }

        response
    }
}
