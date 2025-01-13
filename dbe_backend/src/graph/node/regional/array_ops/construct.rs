use crate::etype::EDataType;
use crate::graph::node::regional::array_ops::{array_op_io, ArrayOpRepeatNode};
use crate::graph::node::regional::{remember_variables, RegionIoKind};
use crate::graph::node::variables::ExecutionExtras;
use crate::graph::node::{ExecutionResult, NodeContext};
use crate::graph::region::{get_region_execution_data, RegionExecutionData};
use crate::value::EValue;
use ustr::Ustr;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ConstructListNode {
    output_ty: Option<EDataType>,
}

impl ArrayOpRepeatNode for ConstructListNode {
    fn id() -> Ustr {
        "construct_list".into()
    }

    fn input_names(&self, kind: RegionIoKind) -> &[&str] {
        match kind {
            RegionIoKind::Start => &["indices"],
            RegionIoKind::End => &["value"],
        }
    }

    fn output_names(&self, kind: RegionIoKind) -> &[&str] {
        match kind {
            RegionIoKind::Start => &["index"],
            RegionIoKind::End => &["values"],
        }
    }

    array_op_io! {
        inputs {
            Start => [Fixed(EDataType::Number)],
            End => [Value(self.output_ty)]
        }
    }

    array_op_io! {
        outputs {
            Start => [2; Fixed(EDataType::Number)],
            End => [2; List(self.output_ty)]
        }
    }

    fn should_execute(
        &self,
        _context: NodeContext,
        region: Uuid,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<bool> {
        let state = get_region_execution_data::<ConstructNodeState>(region, variables)?;

        Ok(state.current < state.repeats)
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
            let n_repeats = inputs[0].try_as_number()?;
            let state = variables.get_or_init_region_data(region, || ConstructNodeState {
                current: 0,
                repeats: n_repeats.0 as usize,
                output: Vec::with_capacity(n_repeats.0 as usize),
                values: None,
            });

            outputs.clear();
            outputs.push(EValue::from(state.current as f64));

            remember_variables(&mut state.values, &inputs[1..], outputs);

            Ok(ExecutionResult::Done)
        } else {
            let state = get_region_execution_data::<ConstructNodeState>(region, variables)?;

            state.current += 1;
            state.output.push(inputs[0].clone());

            if state.current >= state.repeats {
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
                state.values = Some(inputs[1..].to_vec());
                Ok(ExecutionResult::RerunRegion { region })
            }
        }
    }

    fn categories() -> &'static [&'static str] {
        &["list"]
    }

    fn create() -> Self {
        Self { output_ty: None }
    }
}

#[derive(Debug)]
struct ConstructNodeState {
    current: usize,
    repeats: usize,
    output: Vec<EValue>,
    values: Option<Vec<EValue>>,
}

impl RegionExecutionData for ConstructNodeState {}
