use crate::graph::node::commands::SnarlCommands;
use crate::graph::node::extras::ExecutionExtras;
use crate::graph::node::generic::{GenericNode, GenericNodeField, GenericNodeFieldMut};
use crate::graph::node::{ExecutionResult, NodeContext};
use crate::value::EValue;
use egui_snarl::NodeId;
use ustr::Ustr;

#[derive(Debug, Clone, Hash)]
pub struct Test;

impl GenericNode for Test {
    fn id(&self) -> Ustr {
        TestFactory.id()
    }

    fn input_names(&self) -> &[&str] {
        todo!()
    }

    fn output_names(&self) -> &[&str] {
        todo!()
    }

    fn inputs(&self) -> impl AsRef<[GenericNodeField]> {
        todo!()
    }

    fn outputs(&self) -> impl AsRef<[GenericNodeField]> {
        todo!()
    }

    fn inputs_mut(&mut self) -> impl AsMut<[GenericNodeFieldMut]> {
        todo!()
    }

    fn outputs_mut(&mut self) -> impl AsMut<[GenericNodeFieldMut]> {
        todo!()
    }

    fn types_changed(&mut self, context: NodeContext, node: NodeId, commands: &mut SnarlCommands) {
        todo!()
    }

    fn execute(
        &self,
        context: NodeContext,
        inputs: &[EValue],
        outputs: &mut Vec<EValue>,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<ExecutionResult> {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub struct TestFactory;

impl NodeFactory for TestFactory {
    fn id(&self) -> Ustr {
        todo!()
    }

    fn categories(&self) -> &'static [&'static str] {
        todo!()
    }

    fn create(&self) -> Box<dyn Node> {
        Box::new(Test)
    }
}
