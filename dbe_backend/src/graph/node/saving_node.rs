use crate::etype::eitem::EItemInfo;
use crate::etype::EDataType;
use crate::graph::node::{
    impl_serde_node, ExecutionExtras, InputData, Node, NodeContext, NodeFactory, OutputData,
    SnarlNode,
};
use crate::project::side_effects::SideEffect;
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use ustr::Ustr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavingNode {
    pub path: Option<Utf8PathBuf>,
}

impl Node for SavingNode {
    impl_serde_node!();

    fn id(&self) -> Ustr {
        SavingNodeFactory.id()
    }

    fn inputs_count(&self, _context: NodeContext) -> usize {
        1
    }

    fn input_unchecked(&self, context: NodeContext, input: usize) -> miette::Result<InputData> {
        if input != 0 {
            panic!("Saving node has only one input")
        }

        Ok(InputData {
            ty: EItemInfo::simple_type(EDataType::Object {
                ident: context.registry.project_config().types_config.import,
            })
            .into(),
            name: "item".into(),
        })
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
        match &self.path {
            None => variables.side_effects.push(SideEffect::EmitTransientFile {
                value: inputs[0].clone(),
            })?,
            Some(path) => variables
                .side_effects
                .push(SideEffect::EmitPersistentFile {
                    value: inputs[0].clone(),
                    path: path.clone(),
                })?,
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
        Box::new(SavingNode { path: None })
    }
}
