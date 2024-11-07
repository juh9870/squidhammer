use crate::m_try;
use crate::widgets::report::diagnostic_widget;
use crate::workspace::editors::editor_for_value;
use dbe2::diagnostic::context::DiagnosticContextRef;
use dbe2::diagnostic::prelude::{Diagnostic, DiagnosticLevel};
use dbe2::etype::econst::ETypeConst;
use dbe2::etype::EDataType;
use dbe2::graph::execution::partial::PartialGraphExecutionContext;
use dbe2::graph::node::{all_node_factories, SnarlNode};
use dbe2::registry::ETypesRegistry;
use eframe::emath::Pos2;
use egui::{Color32, Stroke, Ui};
use egui_snarl::ui::{PinInfo, SnarlViewer};
use egui_snarl::{InPin, NodeId, OutPin, Snarl};
use miette::miette;
use random_color::options::Luminosity;
use random_color::RandomColor;

pub struct GraphViewer<'a> {
    pub ctx: PartialGraphExecutionContext<'a>,
    pub diagnostics: DiagnosticContextRef<'a>,
}

impl<'a> SnarlViewer<SnarlNode> for GraphViewer<'a> {
    fn title(&mut self, node: &SnarlNode) -> String {
        node.title()
    }

    fn show_header(
        &mut self,
        node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<SnarlNode>,
    ) {
        ui.label(node.0.to_string());
        ui.label(snarl[node].title());
    }

    fn outputs(&mut self, node: &SnarlNode) -> usize {
        node.outputs_count(self.ctx.registry)
    }

    fn inputs(&mut self, node: &SnarlNode) -> usize {
        node.inputs_count(self.ctx.registry)
    }

    fn show_input(
        &mut self,
        pin: &InPin,
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<SnarlNode>,
    ) -> PinInfo {
        m_try(|| {
            let registry = self.ctx.registry;
            let node = &snarl[pin.id.node];
            let input_data = node.try_input(self.ctx.registry, pin.id.input)?;
            if pin.remotes.is_empty() {
                let value = self
                    .ctx
                    .inputs
                    .get_mut(&pin.id)
                    .ok_or_else(|| miette!("Input not found"))?;
                let editor = editor_for_value(self.ctx.registry, value);
                let res = editor.show(
                    ui,
                    self.ctx.registry,
                    self.diagnostics.enter_field(input_data.name.as_str()),
                    &input_data.name,
                    value,
                );

                if res.changed {
                    self.ctx.mark_dirty(snarl, pin.id.node);
                }
            } else {
                let value = self.ctx.read_input(snarl, pin.id)?;
                ui.horizontal(|ui| {
                    ui.label(&*input_data.name);
                    ui.label(value.to_string());
                });
            }

            Ok(pin_info(input_data.ty, registry))
        })
        .unwrap_or_else(|err| {
            ui.set_max_width(128.0);
            diagnostic_widget(
                ui,
                &Diagnostic {
                    info: err,
                    level: DiagnosticLevel::Error,
                },
            );
            PinInfo::circle().with_fill(Color32::BLACK)
        })
    }

    fn show_output(
        &mut self,
        pin: &OutPin,
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<SnarlNode>,
    ) -> PinInfo {
        m_try(|| {
            let registry = self.ctx.registry;
            let node = &snarl[pin.id.node];
            let output_data = node.try_output(self.ctx.registry, pin.id.output)?;
            let value = self.ctx.read_output(snarl, pin.id)?;
            ui.horizontal(|ui| {
                ui.label(&*output_data.name);
                ui.label(value.to_string());
            });

            Ok(pin_info(output_data.ty, registry))
        })
        .unwrap_or_else(|err| {
            ui.set_max_width(128.0);
            diagnostic_widget(
                ui,
                &Diagnostic {
                    info: err,
                    level: DiagnosticLevel::Error,
                },
            );
            PinInfo::circle().with_fill(Color32::BLACK)
        })
    }

    fn has_graph_menu(&mut self, _pos: Pos2, _snarl: &mut Snarl<SnarlNode>) -> bool {
        true
    }

    fn show_graph_menu(
        &mut self,
        pos: Pos2,
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<SnarlNode>,
    ) {
        ui.menu_button("Add node", |ui| {
            for factory in all_node_factories() {
                if ui.button(factory.id().as_str()).clicked() {
                    let node = factory.create();
                    snarl.insert_node(pos, node);
                    ui.close_menu();
                }
            }
        });
    }

    fn connect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<SnarlNode>) {
        if !to.remotes.is_empty() {
            return;
        }

        let from_ty = snarl[from.id.node]
            .try_output(self.ctx.registry, from.id.output)
            .expect("should have output")
            .ty;
        let to_ty = snarl[to.id.node]
            .try_input(self.ctx.registry, to.id.input)
            .expect("should have input")
            .ty;

        if from_ty == to_ty {
            snarl.connect(from.id, to.id);
            self.ctx.mark_dirty(snarl, to.id.node);
        }
    }

    fn disconnect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<SnarlNode>) {
        snarl.disconnect(from.id, to.id);
        self.ctx.mark_dirty(snarl, to.id.node);
    }

    fn drop_inputs(&mut self, pin: &InPin, snarl: &mut Snarl<SnarlNode>) {
        snarl.drop_inputs(pin.id);
        self.ctx.mark_dirty(snarl, pin.id.node);
    }

    fn drop_outputs(&mut self, pin: &OutPin, snarl: &mut Snarl<SnarlNode>) {
        snarl.drop_outputs(pin.id);
        for in_pin in &pin.remotes {
            self.ctx.mark_dirty(snarl, in_pin.node);
        }
    }
}

fn pin_color(ty: EDataType, registry: &ETypesRegistry) -> Color32 {
    const NUMBER_COLOR: Color32 = Color32::from_rgb(161, 161, 161);
    const BOOLEAN_COLOR: Color32 = Color32::from_rgb(204, 166, 214);
    const STRING_COLOR: Color32 = Color32::from_rgb(112, 178, 255);
    const NULL_COLOR: Color32 = Color32::from_rgb(0, 0, 0);
    match ty {
        EDataType::Number => NUMBER_COLOR,
        EDataType::String => STRING_COLOR,
        EDataType::Boolean => BOOLEAN_COLOR,
        EDataType::Const { value } => match value {
            ETypeConst::String(_) => STRING_COLOR,
            ETypeConst::Number(_) => NUMBER_COLOR,
            ETypeConst::Boolean(_) => BOOLEAN_COLOR,
            ETypeConst::Null => NULL_COLOR,
        },
        EDataType::Object { ident } => match registry.get_object(&ident) {
            None => NULL_COLOR,
            Some(data) => data
                .extra_properties()
                .get("pin_color")
                .and_then(|v| v.as_string())
                .and_then(|c| csscolorparser::parse(&c).ok())
                .map(|c| {
                    let rgba = c.to_rgba8();
                    Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3])
                })
                .unwrap_or_else(|| {
                    let c = RandomColor::new()
                        .seed(ident.to_string())
                        .luminosity(Luminosity::Light)
                        .alpha(1.0)
                        .to_rgb_array();
                    Color32::from_rgb(c[0], c[1], c[2])
                }),
        },
        EDataType::List { id } => registry
            .get_list(&id)
            .map(|e| pin_color(e.value_type, registry))
            .unwrap_or(NULL_COLOR),
        EDataType::Map { id } => registry
            .get_map(&id)
            .map(|e| pin_color(e.value_type, registry))
            .unwrap_or(NULL_COLOR),
    }
}

fn pin_stroke(ty: EDataType, registry: &ETypesRegistry) -> Stroke {
    if let EDataType::Map { id } = ty {
        let color = registry
            .get_map(&id)
            .map(|e| pin_color(e.key_type, registry))
            .unwrap_or_else(|| pin_color(ty, registry));
        Stroke::new(4.0, color)
    } else {
        Stroke::NONE
    }
}

fn pin_info(ty: EDataType, registry: &ETypesRegistry) -> PinInfo {
    let shape = match ty {
        EDataType::Boolean | EDataType::Number | EDataType::String | EDataType::Const { .. } => {
            PinInfo::circle()
        }
        EDataType::Object { .. } => PinInfo::circle(),
        EDataType::List { .. } => PinInfo::square(),
        EDataType::Map { .. } => PinInfo::star(),
    };

    shape
        .with_fill(pin_color(ty, registry))
        .with_stroke(pin_stroke(ty, registry))
}
