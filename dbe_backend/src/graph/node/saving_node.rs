use crate::etype::eitem::EItemInfo;
use crate::etype::EDataType;
use crate::graph::node::{
    ExecutionExtras, InputData, Node, NodeContext, NodeFactory, OutputData, SnarlNode,
};
use crate::project::side_effects::SideEffect;
use crate::registry::OPTIONAL_STRING_ID;
use crate::value::EValue;
use miette::bail;
use serde::{Deserialize, Serialize};
use ustr::Ustr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavingNode;

impl Node for SavingNode {
    // impl_serde_node!();

    fn id(&self) -> Ustr {
        SavingNodeFactory.id()
    }

    fn inputs_count(&self, _context: NodeContext) -> usize {
        2
    }

    fn input_unchecked(&self, context: NodeContext, input: usize) -> miette::Result<InputData> {
        match input {
            0 => Ok(InputData::new(
                EItemInfo::simple_type(EDataType::Object {
                    ident: *OPTIONAL_STRING_ID,
                })
                .into(),
                "path".into(),
            )),
            1 => Ok(InputData::new(
                EItemInfo::simple_type(EDataType::Object {
                    ident: context.registry.project_config().types_config.import,
                })
                .into(),
                "item".into(),
            )),
            _ => panic!("Saving node has only two inputs"),
        }
    }

    fn outputs_count(&self, _context: NodeContext) -> usize {
        0
    }

    fn output_unchecked(
        &self,
        _context: NodeContext,
        _output: usize,
    ) -> miette::Result<OutputData> {
        panic!("Saving node has no outputs")
    }

    fn has_side_effects(&self) -> bool {
        true
    }

    fn execute(
        &self,
        _context: NodeContext,
        inputs: &[EValue],
        _outputs: &mut Vec<EValue>,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<()> {
        let EValue::Enum { variant, data } = &inputs[0] else {
            bail!("path input must be an enum, got {:?}", inputs[0]);
        };
        if variant.enum_id() != *OPTIONAL_STRING_ID {
            bail!("path input must be optional string, got {:?}", variant);
        }
        let value = inputs[1].clone();

        if data.is_null() {
            variables
                .side_effects
                .push(SideEffect::EmitTransientFile { value })?;
        } else {
            let path = data.try_as_string()?;
            variables
                .side_effects
                .push(SideEffect::EmitPersistentFile {
                    value: inputs[0].clone(),
                    path: path.into(),
                })?
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SavingNodeFactory;

impl NodeFactory for SavingNodeFactory {
    fn id(&self) -> Ustr {
        "write_item".into()
    }

    fn categories(&self) -> &'static [&'static str] {
        &["output"]
    }

    fn create(&self) -> SnarlNode {
        Box::new(SavingNode)
    }
}
