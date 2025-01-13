use crate::etype::EDataType;
use crate::graph::node::commands::{SnarlCommand, SnarlCommands};
use crate::graph::node::regional::array_ops::{ArrayOpField, ArrayOpFieldMut, ArrayOpRepeatNode};
use crate::graph::node::regional::{remember_variables, RegionIONode, RegionIoKind};
use crate::graph::node::variables::ExecutionExtras;
use crate::graph::node::{ExecutionResult, NodeContext};
use crate::graph::region::{get_region_execution_data, RegionExecutionData};
use crate::value::{ENumber, EValue};
use egui_snarl::NodeId;
use itertools::Itertools;
use miette::bail;
use smallvec::smallvec;
use ustr::Ustr;
use utils::smallvec_n;
use uuid::Uuid;

pub type ListForEachNode = ForEachLikeRegionalNode<{ ForEachKind::ForEach as u8 }>;
pub type ListMapNode = ForEachLikeRegionalNode<{ ForEachKind::Map as u8 }>;
pub type ListFilterNode = ForEachLikeRegionalNode<{ ForEachKind::Filter as u8 }>;
pub type ListFilterMapNode = ForEachLikeRegionalNode<{ ForEachKind::FilterMap as u8 }>;

#[derive(Debug, Clone)]
pub struct ForEachLikeRegionalNode<const KIND: u8> {
    input_ty: Option<EDataType>,
    output_ty: Option<EDataType>,
}

impl<const KIND: u8> ForEachLikeRegionalNode<KIND> {
    #[inline(always)]
    const fn kind(&self) -> ForEachKind {
        ForEachKind::of(KIND)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
#[repr(u8)]
enum ForEachKind {
    ForEach = 0,
    Map,
    Filter,
    FilterMap,
}

impl ForEachKind {
    const fn of(kind: u8) -> Self {
        match kind {
            0 => Self::ForEach,
            1 => Self::Map,
            2 => Self::Filter,
            3 => Self::FilterMap,
            _ => unreachable!(),
        }
    }
}

fn is_map(kind: u8) -> bool {
    kind == ForEachKind::Map as u8 || kind == ForEachKind::FilterMap as u8
}

fn is_filter(kind: u8) -> bool {
    kind == ForEachKind::Filter as u8 || kind == ForEachKind::FilterMap as u8
}

impl PartialEq<u8> for ForEachKind {
    fn eq(&self, other: &u8) -> bool {
        *self as u8 == *other
    }
}

impl PartialEq<ForEachKind> for u8 {
    fn eq(&self, other: &ForEachKind) -> bool {
        *self == *other as u8
    }
}

impl<const KIND: u8> ArrayOpRepeatNode for ForEachLikeRegionalNode<KIND> {
    fn id() -> Ustr {
        match ForEachKind::of(KIND) {
            ForEachKind::ForEach => "for_each".into(),
            ForEachKind::Map => "list_map".into(),
            ForEachKind::Filter => "list_filter".into(),
            ForEachKind::FilterMap => "list_filter_map".into(),
        }
    }

    fn input_names(&self, kind: RegionIoKind) -> &[&str] {
        match kind {
            RegionIoKind::Start => &["values"],
            RegionIoKind::End => match self.kind() {
                ForEachKind::ForEach => &[],
                ForEachKind::Map => &["value"],
                ForEachKind::Filter => &["condition"],
                ForEachKind::FilterMap => &["value", "condition"],
            },
        }
    }

    fn output_names(&self, kind: RegionIoKind) -> &[&str] {
        match kind {
            RegionIoKind::Start => &["value", "index"],
            RegionIoKind::End => {
                if is_map(KIND) || is_filter(KIND) {
                    &["values"]
                } else {
                    &[]
                }
            }
        }
    }

    // array_op_io! {
    //     inputs {
    //         Start => [List(self.input_ty)],
    //         End => [Value(self.output_ty)]
    //     }
    // }

    // array_op_io! {
    //     outputs {
    //         Start => [2; Value(self.input_ty), Fixed(EDataType::Number)],
    //         End => [2; List(self.output_ty)]
    //     }
    // }

    fn inputs(&self, kind: RegionIoKind) -> impl AsRef<[ArrayOpField]> {
        match kind {
            RegionIoKind::Start => smallvec_n![2;ArrayOpField::List(&self.input_ty)],
            RegionIoKind::End => match self.kind() {
                ForEachKind::ForEach => smallvec![],
                ForEachKind::Map => smallvec![ArrayOpField::Value(&self.output_ty)],
                ForEachKind::Filter => smallvec![ArrayOpField::Fixed(EDataType::Boolean)],
                ForEachKind::FilterMap => smallvec![
                    ArrayOpField::Value(&self.output_ty),
                    ArrayOpField::Fixed(EDataType::Boolean)
                ],
            },
        }
    }

    fn outputs(&self, kind: RegionIoKind) -> impl AsRef<[ArrayOpField]> {
        match kind {
            RegionIoKind::Start => {
                smallvec_n![2;ArrayOpField::Value(&self.input_ty), ArrayOpField::Fixed(EDataType::Number)]
            }
            RegionIoKind::End => {
                if is_map(KIND) {
                    smallvec![ArrayOpField::List(&self.output_ty)]
                } else if KIND == ForEachKind::Filter {
                    smallvec![ArrayOpField::List(&self.input_ty)]
                } else {
                    smallvec![]
                }
            }
        }
    }

    fn inputs_mut(&mut self, kind: RegionIoKind) -> impl AsMut<[ArrayOpFieldMut]> {
        match kind {
            RegionIoKind::Start => smallvec_n![2;ArrayOpFieldMut::List(&mut self.input_ty)],
            RegionIoKind::End => match self.kind() {
                ForEachKind::ForEach => smallvec![],
                ForEachKind::Map => smallvec![ArrayOpFieldMut::Value(&mut self.output_ty)],
                ForEachKind::Filter => smallvec![ArrayOpFieldMut::Fixed(EDataType::Boolean)],
                ForEachKind::FilterMap => smallvec![
                    ArrayOpFieldMut::Value(&mut self.output_ty),
                    ArrayOpFieldMut::Fixed(EDataType::Boolean)
                ],
            },
        }
    }

    fn outputs_mut(&mut self, kind: RegionIoKind) -> impl AsMut<[ArrayOpFieldMut]> {
        match kind {
            RegionIoKind::Start => {
                smallvec_n![2;ArrayOpFieldMut::Value(&mut self.input_ty), ArrayOpFieldMut::Fixed(EDataType::Number)]
            }
            RegionIoKind::End => {
                if is_map(KIND) {
                    smallvec![ArrayOpFieldMut::List(&mut self.output_ty)]
                } else if KIND == ForEachKind::Filter {
                    smallvec![ArrayOpFieldMut::List(&mut self.input_ty)]
                } else {
                    smallvec![]
                }
            }
        }
    }

    fn state_changed(
        &mut self,
        context: NodeContext,
        kind: RegionIoKind,
        region: Uuid,
        _node: NodeId,
        commands: &mut SnarlCommands,
    ) {
        if KIND != ForEachKind::Filter {
            return;
        }

        let Some(ty) = self.input_ty else {
            return;
        };

        let Ok(data) = context.region_graph.try_as_data() else {
            self.input_ty = None;
            return;
        };

        let other_id = if kind.is_start() {
            data.region_data(&region).end_node
        } else {
            data.region_data(&region).start_node
        };

        commands.push(SnarlCommand::Custom {
            cb: Box::new(move |ctx| {
                let other = ctx.snarl[other_id]
                    .downcast_mut::<RegionIONode<Self>>()
                    .unwrap();
                other.node.input_ty = Some(ty);
                Ok(())
            }),
        })
    }

    fn should_execute(
        &self,
        _context: NodeContext,
        region: Uuid,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<bool> {
        let state = get_region_execution_data::<ForEachNodeState>(region, variables)?;

        Ok(state.index < state.length)
    }

    fn execute(
        &self,
        context: NodeContext,
        kind: RegionIoKind,
        region: Uuid,
        inputs: &[EValue],
        outputs: &mut Vec<EValue>,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<ExecutionResult> {
        if kind.is_start() {
            let EValue::List { values, .. } = &inputs[0] else {
                bail!("Expected list input, got: {}", inputs[0].ty().name());
            };
            let state = variables.get_or_init_region_data(region, || ForEachNodeState {
                index: 0,
                length: values.len(),
                output: Vec::with_capacity(values.len()),
                input_value: None,
                values: None,
            });

            outputs.clear();
            if !values.is_empty() {
                outputs.push(values[state.index].clone());
                if KIND == ForEachKind::Filter {
                    state.input_value = Some(values[state.index].clone());
                }
            }
            outputs.push(ENumber::from(state.index as f64).into());

            remember_variables(&mut state.values, &inputs[1..], outputs);

            // if !MAP && !FILTER {
            //     debug!(?outputs, state.index, state.length, "ForEachNode start");
            // }

            Ok(ExecutionResult::Done)
        } else {
            let state = get_region_execution_data::<ForEachNodeState>(region, variables)?;

            if state.length > 0 {
                match self.kind() {
                    ForEachKind::ForEach => {}
                    ForEachKind::Map => {
                        state.output.push(inputs[0].clone());
                    }
                    ForEachKind::Filter => {
                        if *inputs[0].try_as_boolean()? {
                            state.output.push(
                                state
                                    .input_value
                                    .take()
                                    .expect("Filter node should have input value"),
                            );
                        }
                    }
                    ForEachKind::FilterMap => {
                        if *inputs[1].try_as_boolean()? {
                            state.output.push(inputs[0].clone());
                        }
                    }
                }
            }
            state.index += 1;

            let skip_n = match self.kind() {
                ForEachKind::ForEach => 0,
                ForEachKind::Map => 1,
                ForEachKind::Filter => 1,
                ForEachKind::FilterMap => 2,
            };
            if state.index >= state.length {
                outputs.clear();
                if is_map(KIND) {
                    outputs.push(EValue::List {
                        values: std::mem::take(&mut state.output),
                        id: context
                            .registry
                            .list_id_of(self.output_ty.unwrap_or_else(EDataType::null)),
                    });
                } else if is_filter(KIND) {
                    outputs.push(EValue::List {
                        values: std::mem::take(&mut state.output),
                        id: context
                            .registry
                            .list_id_of(self.input_ty.unwrap_or_else(EDataType::null)),
                    });
                }
                outputs.extend(inputs.iter().skip(skip_n).cloned());
                variables.remove_region_data(region);
                Ok(ExecutionResult::Done)
            } else {
                state.values = Some(inputs.iter().skip(skip_n).cloned().collect_vec());
                Ok(ExecutionResult::RerunRegion { region })
            }
        }
    }

    fn categories() -> &'static [&'static str] {
        &["utility"]
    }

    fn create() -> Self {
        Self {
            input_ty: None,
            output_ty: None,
        }
    }
}

#[derive(Debug)]
struct ForEachNodeState {
    index: usize,
    length: usize,
    output: Vec<EValue>,
    input_value: Option<EValue>,
    values: Option<Vec<EValue>>,
}

impl RegionExecutionData for ForEachNodeState {}
