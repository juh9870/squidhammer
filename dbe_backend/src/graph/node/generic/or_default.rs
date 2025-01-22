use crate::etype::EDataType;
use crate::graph::node::extras::ExecutionExtras;
use crate::graph::node::generic::macros::generic_node_io;
use crate::graph::node::generic::GenericNode;
use crate::graph::node::{ExecutionResult, Node, NodeContext, NodeFactory};
use crate::registry::optional_helpers::unwrap_optional_value;
use crate::value::EValue;
use ustr::Ustr;

#[derive(Debug, Clone, Hash)]
pub struct OptOrDefault {
    ty: Option<EDataType>,
}

impl GenericNode for OptOrDefault {
    fn id(&self) -> Ustr {
        OptOrDefaultFactory.id()
    }

    fn input_names(&self) -> &[&str] {
        &["value"]
    }

    fn output_names(&self) -> &[&str] {
        &["value"]
    }

    generic_node_io! {
        inputs() {
            [Option(self.ty)]
        }
    }
    generic_node_io! {
        outputs() {
            [Value(self.ty)]
        }
    }

    fn execute(
        &self,
        context: NodeContext,
        inputs: &[EValue],
        outputs: &mut Vec<EValue>,
        _variables: &mut ExecutionExtras,
    ) -> miette::Result<ExecutionResult> {
        let value = if let Some(value) = unwrap_optional_value(context.registry, &inputs[0])? {
            value.clone()
        } else {
            self.ty
                .unwrap_or_else(EDataType::null)
                .default_value(context.registry)
                .into_owned()
        };

        outputs.clear();
        outputs.push(value);

        Ok(ExecutionResult::Done)
    }
}

#[derive(Debug, Clone)]
pub struct OptOrDefaultFactory;

impl NodeFactory for OptOrDefaultFactory {
    fn id(&self) -> Ustr {
        "optional_or_default".into()
    }

    fn categories(&self) -> &'static [&'static str] {
        &["optional"]
    }

    fn create(&self) -> Box<dyn Node> {
        Box::new(OptOrDefault { ty: None })
    }
}
