use crate::etype::EDataType;
use crate::graph::node::generic::{GenericNodeField, GenericNodeFieldMut};
use crate::graph::node::regional::{NodeWithVariables, RegionIoKind, RegionVariableSide};
use crate::graph::node::stateful::generic::GenericStatefulNode;
use crate::graph::node::variables::ExecutionExtras;
use crate::graph::node::{ExecutionResult, NodeContext};
use crate::graph::region::{get_region_execution_data, RegionExecutionData};
use crate::registry::optional_helpers::{none_of_type, unwrap_optional_value, wrap_in_some};
use crate::value::EValue;
use smallvec::smallvec;
use ustr::Ustr;
use utils::smallvec_n;
use uuid::Uuid;

pub type ConditionalIfNode = ConditionalNode<{ ConditionalKind::If as u8 }>;
pub type ConditionalMapNode = ConditionalNode<{ ConditionalKind::Map as u8 }>;

#[derive(Debug, Clone, Hash)]
pub struct ConditionalNode<const KIND: u8> {
    input_ty: Option<EDataType>,
}

impl<const KIND: u8> ConditionalNode<KIND> {
    #[inline(always)]
    const fn kind(&self) -> ConditionalKind {
        ConditionalKind::of(KIND)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
#[repr(u8)]
enum ConditionalKind {
    If = 0,
    Map = 1,
}

impl ConditionalKind {
    const fn of(kind: u8) -> Self {
        match kind {
            0 => Self::If,
            1 => Self::Map,
            _ => unreachable!(),
        }
    }
}
impl<const KIND: u8> NodeWithVariables for ConditionalNode<KIND> {
    fn allow_variables() -> RegionVariableSide {
        RegionVariableSide::END_IN | RegionVariableSide::END_OUT
    }

    fn output_variable_type<'a>(
        &self,
        _context: NodeContext,
        kind: &RegionIoKind,
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
        kind: &RegionIoKind,
        ty: &'a mut Option<EDataType>,
    ) -> GenericNodeFieldMut<'a> {
        if !kind.is_end() {
            panic!("Repeat node has no variables on the start")
        }

        GenericNodeFieldMut::Option(ty)
    }
}

impl<const KIND: u8> GenericStatefulNode for ConditionalNode<KIND> {
    type State = RegionIoKind;

    fn id() -> Ustr {
        match ConditionalKind::of(KIND) {
            ConditionalKind::If => "conditional".into(),
            ConditionalKind::Map => "option_map".into(),
        }
    }

    fn input_names(&self, kind: &RegionIoKind) -> &[&str] {
        if !kind.is_start() {
            panic!("Conditional node has no inputs on the end")
        }

        match self.kind() {
            ConditionalKind::If => &["condition"],
            ConditionalKind::Map => &["option"],
        }
    }

    fn output_names(&self, kind: &RegionIoKind) -> &[&str] {
        if !kind.is_start() {
            panic!("Conditional node has no outputs on the end")
        }
        match self.kind() {
            ConditionalKind::If => &[],
            ConditionalKind::Map => &["value"],
        }
    }

    fn inputs(&self, kind: &RegionIoKind) -> impl AsRef<[GenericNodeField]> {
        match kind {
            RegionIoKind::Start => match self.kind() {
                ConditionalKind::If => smallvec_n![1;GenericNodeField::Fixed(EDataType::Boolean)],
                ConditionalKind::Map => smallvec![GenericNodeField::Option(&self.input_ty)],
            },
            RegionIoKind::End => smallvec![],
        }
    }

    fn outputs(&self, kind: &RegionIoKind) -> impl AsRef<[GenericNodeField]> {
        match kind {
            RegionIoKind::Start => match self.kind() {
                ConditionalKind::If => smallvec![],
                ConditionalKind::Map => smallvec_n![1;GenericNodeField::Value(&self.input_ty)],
            },
            RegionIoKind::End => smallvec![],
        }
    }

    fn inputs_mut(&mut self, kind: &RegionIoKind) -> impl AsMut<[GenericNodeFieldMut]> {
        match kind {
            RegionIoKind::Start => match self.kind() {
                ConditionalKind::If => {
                    smallvec_n![1;GenericNodeFieldMut::Fixed(EDataType::Boolean)]
                }
                ConditionalKind::Map => smallvec![GenericNodeFieldMut::Option(&mut self.input_ty)],
            },
            RegionIoKind::End => smallvec![],
        }
    }

    fn outputs_mut(&mut self, kind: &RegionIoKind) -> impl AsMut<[GenericNodeFieldMut]> {
        match kind {
            RegionIoKind::Start => match self.kind() {
                ConditionalKind::If => smallvec![],
                ConditionalKind::Map => {
                    smallvec_n![1;GenericNodeFieldMut::Value(&mut self.input_ty)]
                }
            },
            RegionIoKind::End => smallvec![],
        }
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
        kind: &RegionIoKind,
        region: Uuid,
        inputs: &[EValue],
        outputs: &mut Vec<EValue>,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<ExecutionResult> {
        outputs.clear();

        if kind.is_start() {
            match ConditionalKind::of(KIND) {
                ConditionalKind::If => {
                    let condition = *inputs[0].try_as_boolean()?;
                    variables
                        .get_or_init_region_data(region, |_| ConditionalNodeState { condition });
                }
                ConditionalKind::Map => {
                    let value = unwrap_optional_value(context.registry, &inputs[0])?;
                    let condition = value.is_some();
                    variables
                        .get_or_init_region_data(region, |_| ConditionalNodeState { condition });
                    if let Some(value) = value {
                        outputs.push(value.clone())
                    }
                }
            }
            Ok(ExecutionResult::Done)
        } else {
            let state = get_region_execution_data::<ConditionalNodeState>(region, variables)?;

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
        Self { input_ty: None }
    }
}

#[derive(Debug)]
struct ConditionalNodeState {
    condition: bool,
}

impl RegionExecutionData for ConditionalNodeState {}
