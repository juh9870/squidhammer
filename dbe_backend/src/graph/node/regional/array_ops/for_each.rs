use crate::etype::EDataType;
use crate::graph::node::regional::array_ops::{array_op_io, ArrayOpRepeatNode};
use crate::graph::node::regional::{remember_variables, RegionIoKind};
use crate::graph::node::variables::ExecutionExtras;
use crate::graph::node::{ExecutionResult, NodeContext};
use crate::graph::region::{get_region_execution_data, RegionExecutionData};
use crate::value::{ENumber, EValue};
use itertools::Itertools;
use miette::bail;
use ustr::Ustr;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ForEachRegionalNode {
    input_ty: Option<EDataType>,
    output_ty: Option<EDataType>,
}

impl ArrayOpRepeatNode for ForEachRegionalNode {
    fn id() -> Ustr {
        "for_each".into()
    }

    fn input_names(&self, kind: RegionIoKind) -> &[&str] {
        match kind {
            RegionIoKind::Start => &["values"],
            RegionIoKind::End => &["value"],
        }
    }

    fn output_names(&self, kind: RegionIoKind) -> &[&str] {
        match kind {
            RegionIoKind::Start => &["value", "index"],
            RegionIoKind::End => &["values"],
        }
    }

    array_op_io! {
        inputs {
            Start => [List(self.input_ty)],
            End => [Value(self.output_ty)]
        }
    }

    array_op_io! {
        outputs {
            Start => [2; Value(self.input_ty), Fixed(EDataType::Number)],
            End => [2; List(self.output_ty)]
        }
    }

    fn should_execute(
        &self,
        _context: NodeContext,
        region: Uuid,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<bool> {
        let Some(state) = variables.get_region_data::<ForEachNodeState>(region) else {
            bail!("End of for-each node without start")
        };

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
                values: None,
            });

            outputs.clear();
            outputs.push(values[state.index].clone());
            outputs.push(ENumber::from(state.index as f64).into());

            remember_variables(&mut state.values, &inputs[1..], outputs);

            Ok(ExecutionResult::Done)
        } else {
            let state = get_region_execution_data::<ForEachNodeState>(region, variables)?;

            state.index += 1;
            state.output.push(inputs[0].clone());

            if state.index >= state.length {
                outputs.clear();
                outputs.push(EValue::List {
                    values: std::mem::take(&mut state.output),
                    id: context
                        .registry
                        .list_id_of(self.output_ty.unwrap_or_else(EDataType::null)),
                });
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
    values: Option<Vec<EValue>>,
}

impl RegionExecutionData for ForEachNodeState {}
