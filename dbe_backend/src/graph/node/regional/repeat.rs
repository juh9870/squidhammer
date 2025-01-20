use crate::etype::eitem::EItemInfo;
use crate::etype::EDataType;
use crate::graph::node::ports::{InputData, OutputData};
use crate::graph::node::regional::{remember_variables, NodeWithVariables, RegionIoKind};
use crate::graph::node::stateful::StatefulNode;
use crate::graph::node::variables::ExecutionExtras;
use crate::graph::node::{ExecutionResult, NodeContext};
use crate::graph::region::RegionExecutionData;
use crate::json_utils::json_serde::JsonSerde;
use crate::value::EValue;
use miette::bail;
use ustr::Ustr;
use uuid::Uuid;

#[derive(Debug, Clone, Hash)]
pub struct RepeatNode;

impl NodeWithVariables for RepeatNode {}

impl JsonSerde for RepeatNode {
    type State<'a> = RegionIoKind;
}

impl StatefulNode for RepeatNode {
    type State<'a> = RegionIoKind;

    fn id() -> Ustr {
        "repeat".into()
    }

    fn inputs_count(&self, _context: NodeContext, kind: RegionIoKind) -> usize {
        if kind.is_start() {
            1
        } else {
            0
        }
    }

    fn outputs_count(&self, _context: NodeContext, kind: RegionIoKind) -> usize {
        if kind.is_start() {
            1
        } else {
            0
        }
    }

    fn input_unchecked(
        &self,
        _context: NodeContext,
        kind: RegionIoKind,
        _input: usize,
    ) -> miette::Result<InputData> {
        if kind.is_start() {
            Ok(InputData::new(
                EItemInfo::simple_type(EDataType::Number).into(),
                "iterations".into(),
            ))
        } else {
            panic!("Repeat node has no inputs on the end")
        }
    }

    fn output_unchecked(
        &self,
        _context: NodeContext,
        kind: RegionIoKind,
        _output: usize,
    ) -> miette::Result<OutputData> {
        if kind.is_start() {
            Ok(OutputData::new(
                EItemInfo::simple_type(EDataType::Number).into(),
                "iteration".into(),
            ))
        } else {
            panic!("Repeat node has no outputs on the end")
        }
    }

    fn should_execute(
        &self,
        _context: NodeContext,
        region: Uuid,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<bool> {
        let Some(state) = variables.get_region_data::<RepeatNodeState>(region) else {
            bail!("End of repeat node without start")
        };

        Ok(state.current < state.repeats)
    }

    fn execute(
        &self,
        _context: NodeContext,
        kind: RegionIoKind,
        region: Uuid,
        inputs: &[EValue],
        outputs: &mut Vec<EValue>,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<ExecutionResult> {
        if kind.is_start() {
            let n_repeats = inputs[0].try_as_number()?;
            let state = variables.get_or_init_region_data(region, |_| RepeatNodeState {
                repeats: n_repeats.0 as u32,
                current: 0,
                values: None,
            });

            outputs.clear();
            outputs.push(EValue::from(state.current as f64));

            remember_variables(&mut state.values, &inputs[1..], outputs);

            Ok(ExecutionResult::Done)
        } else {
            let Some(state) = variables.get_region_data::<RepeatNodeState>(region) else {
                bail!("End of repeat node without start")
            };

            state.current += 1;

            if state.current >= state.repeats {
                outputs.clear();
                outputs.extend(inputs.iter().cloned());
                variables.remove_region_data(region);
                Ok(ExecutionResult::Done)
            } else {
                state.values = Some(inputs.to_vec());
                Ok(ExecutionResult::RerunRegion { region })
            }
        }
    }

    fn categories() -> &'static [&'static str] {
        &["utility", "utility.iterators"]
    }

    fn create() -> Self {
        Self
    }
}

#[derive(Debug)]
struct RepeatNodeState {
    repeats: u32,
    current: u32,
    values: Option<Vec<EValue>>,
}

impl RegionExecutionData for RepeatNodeState {}
