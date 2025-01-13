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

pub type ListForEachNode = ForEachLikeRegionalNode<false, false>;
pub type ListMapNode = ForEachLikeRegionalNode<true, false>;
pub type ListFilterNode = ForEachLikeRegionalNode<false, true>;
pub type ListFilterMapNode = ForEachLikeRegionalNode<true, true>;

#[derive(Debug, Clone)]
pub struct ForEachLikeRegionalNode<const MAP: bool, const FILTER: bool> {
    input_ty: Option<EDataType>,
    output_ty: Option<EDataType>,
}

impl<const MAP: bool, const FILTER: bool> ForEachLikeRegionalNode<MAP, FILTER> {
    #[inline(always)]
    const fn kind(&self) -> ForEachKind {
        ForEachKind::of(MAP, FILTER)
    }
}

#[derive(Debug, Clone, Copy)]
enum ForEachKind {
    ForEach,
    Map,
    Filter,
    FilterMap,
}

impl ForEachKind {
    const fn of(map: bool, filter: bool) -> Self {
        match (map, filter) {
            (false, false) => Self::ForEach,
            (true, false) => Self::Map,
            (false, true) => Self::Filter,
            (true, true) => Self::FilterMap,
        }
    }
}

impl<const MAP: bool, const FILTER: bool> ArrayOpRepeatNode
    for ForEachLikeRegionalNode<MAP, FILTER>
{
    fn id() -> Ustr {
        match ForEachKind::of(MAP, FILTER) {
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
                if MAP || FILTER {
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
                if MAP {
                    smallvec![ArrayOpField::List(&self.output_ty)]
                } else if FILTER {
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
                if MAP {
                    smallvec![ArrayOpFieldMut::List(&mut self.output_ty)]
                } else if FILTER {
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
        if !FILTER {
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
                if FILTER {
                    state.input_value = Some(values[state.index].clone());
                }
            }
            outputs.push(ENumber::from(state.index as f64).into());

            remember_variables(&mut state.values, &inputs[1..], outputs);

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

            if state.index >= state.length {
                outputs.clear();
                if MAP {
                    outputs.push(EValue::List {
                        values: std::mem::take(&mut state.output),
                        id: context
                            .registry
                            .list_id_of(self.output_ty.unwrap_or_else(EDataType::null)),
                    });
                } else if FILTER {
                    outputs.push(EValue::List {
                        values: std::mem::take(&mut state.output),
                        id: context
                            .registry
                            .list_id_of(self.input_ty.unwrap_or_else(EDataType::null)),
                    });
                }
                outputs.extend(inputs.iter().skip(1).cloned());
                variables.remove_region_data(region);
                Ok(ExecutionResult::Done)
            } else {
                state.values = Some(inputs.iter().skip(1).cloned().collect_vec());
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
