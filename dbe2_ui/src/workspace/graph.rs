use crate::error::report_error;
use crate::m_try;
use crate::widgets::report::diagnostic_widget;
use crate::workspace::graph::viewer::get_viewer;
use ahash::AHashMap;
use dbe2::diagnostic::context::DiagnosticContextRef;
use dbe2::diagnostic::prelude::{Diagnostic, DiagnosticLevel};
use dbe2::etype::econst::ETypeConst;
use dbe2::etype::eitem::EItemInfo;
use dbe2::etype::EDataType;
use dbe2::graph::execution::partial::PartialGraphExecutionContext;
use dbe2::graph::node::commands::SnarlCommands;
use dbe2::graph::node::enum_node::EnumNode;
use dbe2::graph::node::struct_node::StructNode;
use dbe2::graph::node::{
    all_node_factories, get_snarl_node, node_factories_by_category, NodeFactory, SnarlNode,
};
use dbe2::registry::{EObjectType, ETypesRegistry};
use dbe2::value::id::ETypeId;
use eframe::emath::Pos2;
use egui::{Color32, Stroke, Ui};
use egui_hooks::UseHookExt;
use egui_snarl::ui::{PinInfo, SnarlViewer};
use egui_snarl::{InPin, NodeId, OutPin, Snarl};
use random_color::options::Luminosity;
use random_color::RandomColor;
use std::iter::Peekable;
use std::ops::DerefMut;
use std::sync::Arc;
use ustr::Ustr;

pub mod viewer;

#[derive(Debug)]
pub struct GraphViewer<'a> {
    pub ctx: PartialGraphExecutionContext<'a>,
    pub diagnostics: DiagnosticContextRef<'a>,
    commands: SnarlCommands,
}

impl<'a> GraphViewer<'a> {
    pub fn new(
        ctx: PartialGraphExecutionContext<'a>,
        diagnostics: DiagnosticContextRef<'a>,
    ) -> Self {
        Self {
            ctx,
            diagnostics,
            commands: Default::default(),
        }
    }
}

impl<'a> SnarlViewer<SnarlNode> for GraphViewer<'a> {
    fn title(&mut self, node: &SnarlNode) -> String {
        node.title()
    }

    fn show_header(
        &mut self,
        node: NodeId,
        inputs: &[InPin],
        outputs: &[OutPin],
        ui: &mut Ui,
        scale: f32,
        snarl: &mut Snarl<SnarlNode>,
    ) {
        m_try(|| {
            get_viewer(&snarl[node].id()).show_header(self, node, inputs, outputs, ui, scale, snarl)
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
        })
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
        m_try(|| get_viewer(&snarl[pin.id.node].id()).show_input(self, pin, ui, _scale, snarl))
            .unwrap_or_else(|err| {
                // ui.set_max_width(128.0);
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
        m_try(|| get_viewer(&snarl[pin.id.node].id()).show_output(self, pin, ui, _scale, snarl))
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

    fn has_body(&mut self, node: &SnarlNode) -> bool {
        m_try(|| get_viewer(&node.id()).has_body(self, node)).unwrap_or_else(|err| {
            report_error(err);
            false
        })
    }

    fn show_body(
        &mut self,
        node: NodeId,
        inputs: &[InPin],
        outputs: &[OutPin],
        ui: &mut Ui,
        scale: f32,
        snarl: &mut Snarl<SnarlNode>,
    ) {
        m_try(|| {
            get_viewer(&snarl[node].id()).show_body(self, node, inputs, outputs, ui, scale, snarl)
        })
        .unwrap_or_else(|err| {
            diagnostic_widget(
                ui,
                &Diagnostic {
                    info: err,
                    level: DiagnosticLevel::Error,
                },
            );
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
            #[derive(Debug, Copy, Clone)]
            enum NodeCombo {
                Factory(Ustr),
                Object(ETypeId),
            }

            impl AsRef<str> for NodeCombo {
                fn as_ref(&self) -> &str {
                    match self {
                        NodeCombo::Factory(id) => id.as_str(),
                        NodeCombo::Object(id) => id.as_raw().unwrap(),
                    }
                }
            }

            let mut search_query = ui.use_state(|| "".to_string(), ()).into_var();
            let search_nodes = ui.use_memo(
                || {
                    let factories = all_node_factories();
                    let all_nodes = factories.iter().map(|(id, _)| NodeCombo::Factory(*id));
                    let objects = self
                        .ctx
                        .registry
                        .all_objects()
                        .filter(|obj| {
                            match obj
                                .extra_properties()
                                .get("graph_search_hide")
                                .and_then(|o| o.as_bool())
                            {
                                None => true,
                                Some(hide) => !hide,
                            }
                        })
                        .map(|s| NodeCombo::Object(s.ident()));
                    all_nodes
                        .chain(objects)
                        .map(|n| (n.as_ref().to_string(), n))
                        .collect::<AHashMap<_, _>>()
                },
                (),
            );

            let search_bar = ui.text_edit_singleline(search_query.deref_mut());
            search_bar.request_focus();

            if search_query.trim() != "" {
                for (name, node) in search_nodes
                    .iter()
                    .filter(|(k, _)| k.contains(search_query.as_str()))
                    .take(10)
                {
                    if ui.button(name).clicked() {
                        let node = match node {
                            NodeCombo::Factory(id) => get_snarl_node(id).unwrap(),
                            NodeCombo::Object(id) => {
                                let node: SnarlNode = match self
                                    .ctx
                                    .registry
                                    .get_object(id)
                                    .expect("object id should be valid")
                                {
                                    EObjectType::Struct(_) => Box::new(StructNode::new(*id)),
                                    EObjectType::Enum(data) => {
                                        Box::new(EnumNode::new(data.variant_ids()[0]))
                                    }
                                };

                                node
                            }
                        };
                        snarl.insert_node(pos, node);
                        ui.close_menu();
                    }
                }
            }

            let categories = node_factories_by_category();
            let mut categories = categories.iter().peekable();

            fn is_sub_category(category: &str, parent: &str) -> bool {
                category.starts_with(parent)
                    && category.chars().nth(parent.len()).is_some_and(|c| c == '.')
            }
            fn show<'a>(
                ui: &mut Ui,
                parent: &'static str,
                categories: &mut Peekable<
                    impl Iterator<Item = (&'a &'static str, &'a Vec<Arc<dyn NodeFactory>>)>,
                >,
            ) -> Option<SnarlNode> {
                while let Some((cat, _)) = categories.peek() {
                    if !parent.is_empty() && !is_sub_category(cat, parent) {
                        return None;
                    }

                    if !parent.is_empty() {
                        ui.separator();
                    }

                    let (category, factories) = categories.next().unwrap();
                    let cat_name = category
                        .strip_prefix(parent)
                        .and_then(|c| c.strip_prefix("."))
                        .unwrap_or(category);
                    if let Some(node) = ui
                        .menu_button(cat_name, |ui| {
                            for factory in factories.iter() {
                                if ui.button(factory.id().as_str()).clicked() {
                                    let node = factory.create();
                                    ui.close_menu();
                                    return Some(node);
                                }
                            }

                            show(ui, category, categories)
                        })
                        .inner
                        .flatten()
                    {
                        return Some(node);
                    }

                    while let Some((next_cat, _)) = categories.peek() {
                        if !is_sub_category(next_cat, category) {
                            break;
                        }
                        categories.next();
                    }
                }

                None
            }

            if let Some(to_insert) = show(ui, "", &mut categories) {
                snarl.insert_node(pos, to_insert);
            }
        });
    }

    fn has_node_menu(&mut self, _node: &SnarlNode) -> bool {
        true
    }

    fn show_node_menu(
        &mut self,
        node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<SnarlNode>,
    ) {
        m_try(|| {
            ui.label("Node menu");
            if ui.button("Remove").clicked() {
                self.ctx.remove_node(node, snarl, &mut self.commands)?;
                ui.close_menu();
            }
            Ok(())
        })
        .unwrap_or_else(report_error)
    }

    fn connect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<SnarlNode>) {
        if let Err(err) = self.ctx.connect(from, to, snarl, &mut self.commands) {
            report_error(err)
        }
    }

    fn disconnect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<SnarlNode>) {
        if let Err(err) = self.ctx.disconnect(from, to, snarl, &mut self.commands) {
            report_error(err);
        }
    }

    fn drop_outputs(&mut self, pin: &OutPin, snarl: &mut Snarl<SnarlNode>) {
        if let Err(err) = m_try(|| {
            for remote in &pin.remotes {
                self.ctx
                    .disconnect(pin, &snarl.in_pin(*remote), snarl, &mut self.commands)?;
            }

            Ok(())
        }) {
            report_error(err);
        }
    }

    fn drop_inputs(&mut self, pin: &InPin, snarl: &mut Snarl<SnarlNode>) {
        if let Err(err) = m_try(|| {
            for remote in &pin.remotes {
                self.ctx
                    .disconnect(&snarl.out_pin(*remote), pin, snarl, &mut self.commands)?;
            }

            Ok(())
        }) {
            report_error(err);
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

fn pin_info(ty: &EItemInfo, registry: &ETypesRegistry) -> PinInfo {
    let shape = match ty.ty() {
        EDataType::Boolean | EDataType::Number | EDataType::String | EDataType::Const { .. } => {
            PinInfo::circle()
        }
        EDataType::Object { .. } => PinInfo::circle(),
        EDataType::List { .. } => PinInfo::square(),
        EDataType::Map { .. } => PinInfo::star(),
    };

    shape
        .with_fill(pin_color(ty.ty(), registry))
        .with_stroke(pin_stroke(ty.ty(), registry))
}
