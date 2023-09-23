use crate::states::main_state::TabHandler;
use egui::{Color32, DragValue, Pos2, Slider, Ui};
use list_edit::list_editor;

pub fn show_mesh_test(
    state: &mut TabHandler,
    ui: &mut Ui,
    points: &mut Vec<(Pos2, Color32)>,
    indices: &mut Vec<u32>,
) {
    ui.horizontal_top(|ui| {
        list_editor("vertices")
            .new_item(|_| (Pos2::ZERO, Color32::RED))
            .show(ui, points, |ui, i, data| {
                ui.label(i.index.to_string());
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("x");
                        ui.add(Slider::new(&mut data.0.x, 0.0..=1.0))
                    });
                    ui.horizontal(|ui| {
                        ui.label("y");
                        ui.add(Slider::new(&mut data.0.y, 0.0..=1.0))
                    });
                    let mut color = data.1.to_array();

                    ui.horizontal(|ui| {
                        ui.label("color");
                        ui.color_edit_button_srgba_premultiplied(&mut color);
                    });
                    data.1 =
                        Color32::from_rgba_premultiplied(color[0], color[1], color[2], color[3]);
                });
            });

        ui.separator();

        list_editor("indices")
            .new_item(|_| 0)
            .show(ui, indices, |ui, _, data| {
                ui.add(DragValue::new(data).clamp_range(0..=points.len()));
            });

        ui.separator();

        Frame::canvas(ui.style()).show(ui, |ui| {
            let (mut response, painter) =
                ui.allocate_painter(ui.available_size_before_wrap(), Sense::drag());

            let to_screen = emath::RectTransform::from_to(
                Rect::from_min_size(Pos2::ZERO, response.rect.square_proportions()),
                response.rect,
            );
            let from_screen = to_screen.inverse();

            // let vert = |color: Color32| {
            //     move |x: f32, y: f32| Vertex {
            //         pos: to_screen * Pos2::new(x, y),
            //         uv: Pos2::ZERO,
            //         color,
            //     }
            // };

            // let red = vert(Color32::RED);
            // let green = vert(Color32::GREEN);
            // let blue = vert(Color32::BLUE);

            let vertices = points
                .iter()
                .map(|(p, c)| Vertex {
                    pos: to_screen * *p,
                    color: *c,
                    uv: Pos2::ZERO,
                })
                .collect_vec();
            let mesh = Mesh {
                indices: indices.clone(),
                vertices,
                texture_id: Default::default(),
            };
            painter.extend([Shape::mesh(mesh)]);
            response.mark_changed();

            response
        });
    });
}
