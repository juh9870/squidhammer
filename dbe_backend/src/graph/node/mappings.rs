use crate::etype::default::DefaultEValue;
use crate::etype::eitem::EItemInfo;
use crate::etype::EDataType;
use crate::graph::node::ports::{InputData, OutputData};
use crate::graph::node::variables::ExecutionExtras;
use crate::graph::node::{ExecutionResult, Node, NodeContext, NodeFactory};
use crate::project::EXTENSION_VALUE;
use crate::value::id::ETypeId;
use crate::value::EValue;
use miette::bail;
use std::sync::LazyLock;
use ustr::Ustr;

pub static KIND_ID: LazyLock<ETypeId> =
    LazyLock::new(|| ETypeId::from_raw("sys:mappings/kind".into()));
pub static RANGE_ID: LazyLock<ETypeId> =
    LazyLock::new(|| ETypeId::from_raw("sys:math/range".into()));

#[derive(Debug, Clone, Hash, Default)]
pub struct MappingsNode {}

impl Node for MappingsNode {
    fn id(&self) -> Ustr {
        MappingsNodeFactory.id()
    }

    fn default_input_value(
        &self,
        context: NodeContext,
        input: usize,
    ) -> miette::Result<DefaultEValue> {
        if input == 2 {
            return Ok(EValue::Boolean { value: true }.into());
        }

        let input = self.try_input(context, input)?;
        Ok(input.ty.default_value(context.registry))
    }

    fn inputs_count(&self, _context: NodeContext) -> usize {
        5
    }

    fn input_unchecked(&self, context: NodeContext, input: usize) -> miette::Result<InputData> {
        match input {
            0 => Ok(InputData::new(
                EItemInfo::simple_type(EDataType::String).into(),
                "path".into(),
            )),
            1 => Ok(InputData::new(
                EItemInfo::simple_type(
                    context
                        .registry
                        .list_of(EDataType::Object { ident: *RANGE_ID }),
                )
                .into(),
                "default_ranges".into(),
            )),
            2 => Ok(InputData::new(
                EItemInfo::simple_type(EDataType::Boolean).into(),
                "persistent".into(),
            )),
            3 => Ok(InputData::new(
                EItemInfo::simple_type(EDataType::Object { ident: *KIND_ID }).into(),
                "kind".into(),
            )),
            4 => Ok(InputData::new(
                EItemInfo::simple_type(EDataType::String).into(),
                "input".into(),
            )),
            _ => {
                panic!("Mappings node has only five inputs");
            }
        }
    }

    fn outputs_count(&self, _context: NodeContext) -> usize {
        1
    }

    fn output_unchecked(&self, _context: NodeContext, output: usize) -> miette::Result<OutputData> {
        if output != 0 {
            panic!("Mappings node has only one output");
        }

        Ok(OutputData::new(
            EItemInfo::simple_type(EDataType::Number).into(),
            "output".into(),
        ))
    }

    fn has_side_effects(&self) -> bool {
        true
    }

    fn execute(
        &self,
        context: NodeContext,
        inputs: &[EValue],
        outputs: &mut Vec<EValue>,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<ExecutionResult> {
        let path = inputs[0].try_as_string()?;
        let ranges = &inputs[1];
        let persistent = &inputs[2].try_as_boolean()?;
        let kind = &inputs[3];
        let value = inputs[4].try_as_string()?;

        let EValue::Enum { data, .. } = kind else {
            bail!("kind input must be an enum, got {:?}", kind);
        };

        let kind_idx = data.try_as_number()?;

        let path = path.trim();

        if path.is_empty() {
            bail!("path must not be empty");
        }

        let path = format!("{}.{}", path, EXTENSION_VALUE);

        let mappings =
            variables
                .side_effects
                .load_mappings(context.registry, path.as_ref(), ranges)?;

        let id = match kind_idx.0 {
            0.0 => mappings.get_id_raw(value.to_string(), **persistent)?,
            1.0 => mappings.new_id(value.to_string(), **persistent)?,
            // 2.0 => mappings.existing_id(value)?,
            2.0 => bail!(
                "existing ID mapping is not yet implemented, blocked due to multistage runtime"
            ), // TODO: allow once multi-stage runtime is implemented
            _ => bail!("invalid kind index: {}", kind_idx.0),
        };

        outputs.clear();
        outputs.push(EValue::Number {
            value: (id as f64).into(),
        });

        Ok(ExecutionResult::Done)
    }
}

#[derive(Debug, Clone)]
pub struct MappingsNodeFactory;

impl NodeFactory for MappingsNodeFactory {
    fn id(&self) -> Ustr {
        "mappings".into()
    }

    fn categories(&self) -> &'static [&'static str] {
        &["utility"]
    }

    fn create(&self) -> Box<dyn Node> {
        Box::new(MappingsNode::default())
    }
}
