use crate::etype::eitem::EItemInfo;
use crate::etype::EDataType;
use crate::graph::node::generic::{GenericNodeField, GenericNodeFieldMut};
use crate::graph::node::ports::{InputData, OutputData};
use crate::graph::node::regional::{RegionIoKind, RegionVariableSide, RegionalNode};
use crate::graph::node::variables::ExecutionExtras;
use crate::graph::node::{ExecutionResult, NodeContext};
use crate::graph::region::{get_region_execution_data, RegionExecutionData};
use crate::registry::optional_helpers::{none_of_type, wrap_in_some};
use crate::value::EValue;
use ustr::Ustr;
use uuid::Uuid;

#[derive(Debug, Clone, Hash)]
pub struct ConditionalNode;

impl RegionalNode for ConditionalNode {
    fn id() -> Ustr {
        "conditional".into()
    }

    fn allow_variables() -> RegionVariableSide {
        RegionVariableSide::END_IN | RegionVariableSide::END_OUT
    }

    fn output_variable_type<'a>(
        &self,
        _context: NodeContext,
        kind: RegionIoKind,
        ty: &'a Option<EDataType>,
    ) -> GenericNodeField<'a> {
        if !kind.is_end() {
            panic!("Repeat node has no variables on the start")
        }

        GenericNodeField::Option(ty)
    }

    fn output_variable_type_mut<'a>(
        &self,
        _context: NodeContext,
        kind: RegionIoKind,
        ty: &'a mut Option<EDataType>,
    ) -> GenericNodeFieldMut<'a> {
        if !kind.is_end() {
            panic!("Repeat node has no variables on the start")
        }

        GenericNodeFieldMut::Option(ty)
    }

    fn inputs_count(&self, _context: NodeContext, kind: RegionIoKind) -> usize {
        if kind.is_start() {
            1
        } else {
            0
        }
    }

    fn outputs_count(&self, _context: NodeContext, _kind: RegionIoKind) -> usize {
        0
    }

    fn input_unchecked(
        &self,
        _context: NodeContext,
        kind: RegionIoKind,
        _input: usize,
    ) -> miette::Result<InputData> {
        if kind.is_start() {
            Ok(InputData::new(
                EItemInfo::simple_type(EDataType::Boolean).into(),
                "condition".into(),
            ))
        } else {
            panic!("Conditional node has no inputs on the end")
        }
    }

    fn output_unchecked(
        &self,
        _context: NodeContext,
        _kind: RegionIoKind,
        _output: usize,
    ) -> miette::Result<OutputData> {
        panic!("Conditional node has no outputs")
    }

    fn should_execute(
        &self,
        _context: NodeContext,
        region: Uuid,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<bool> {
        let state = get_region_execution_data::<ConditionalNodeState>(region, variables)?;

        Ok(state.condition)
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
            let condition = *inputs[0].try_as_boolean()?;
            variables.get_or_init_region_data(region, |_| ConditionalNodeState { condition });

            Ok(ExecutionResult::Done)
        } else {
            let state = get_region_execution_data::<ConditionalNodeState>(region, variables)?;

            outputs.clear();

            if state.condition {
                outputs.extend(
                    inputs
                        .iter()
                        .map(|value| wrap_in_some(context.registry, value.clone())),
                );
            } else {
                outputs.extend(
                    inputs
                        .iter()
                        .map(|value| none_of_type(context.registry, value.ty())),
                );
            }

            variables.remove_region_data(region);
            Ok(ExecutionResult::Done)
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
struct ConditionalNodeState {
    condition: bool,
}

impl RegionExecutionData for ConditionalNodeState {}
