use egui::{Color32, Rect, Ui};
use egui_node_graph::port_shapes::{
    draw_circle_port, draw_diamond_port, draw_hollow_circle_port, draw_rect_port,
};
use knuffel::DecodeScalar;

#[non_exhaustive]
#[derive(Debug, Copy, Clone, DecodeScalar)]
pub enum PortShape {
    Circle,
    Hollow,
    Diamond,
    Rect,
}

impl PortShape {
    pub fn draw_port(
        &self,
        ui: &mut Ui,
        wide_port: bool,
        port_rect: Rect,
        zoom: f32,
        port_color: Color32,
    ) {
        match self {
            PortShape::Circle => draw_circle_port(ui, wide_port, port_rect, zoom, port_color),
            PortShape::Hollow => {
                draw_hollow_circle_port(ui, wide_port, port_rect, zoom, port_color)
            }
            PortShape::Diamond => draw_diamond_port(ui, wide_port, port_rect, zoom, port_color),
            PortShape::Rect => draw_rect_port(ui, wide_port, port_rect, zoom, port_color),
        }
    }
}
