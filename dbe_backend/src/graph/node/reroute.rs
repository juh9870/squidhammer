use crate::etype::eitem::EItemInfo;
use crate::graph::node::commands::{SnarlCommand, SnarlCommands};
use crate::graph::node::ports::NodePortType;
use crate::graph::node::{InputData, Node, NodeFactory, OutputData, SnarlNode};
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use egui_snarl::{InPin, InPinId, OutPin, OutPinId};
use ustr::Ustr;

#[derive(Debug, Clone, Default)]
pub struct RerouteNode {
    inputs: Vec<EItemInfo>,
}
impl Node for RerouteNode {
    fn id(&self) -> Ustr {
        RerouteFactory.id()
    }

    fn has_inline_values(&self) -> miette::Result<bool> {
        Ok(false)
    }

    fn inputs_count(&self, _registry: &ETypesRegistry) -> usize {
        self.inputs.len() + 1
    }

    fn input_unchecked(
        &self,
        _registry: &ETypesRegistry,
        input: usize,
    ) -> miette::Result<InputData> {
        if input == self.inputs.len() {
            return Ok(InputData {
                ty: NodePortType::Any,
                name: "".into(),
            });
        }
        Ok(InputData {
            ty: self.inputs[input].clone().into(),
            name: input.to_string().into(),
        })
    }

    fn outputs_count(&self, _registry: &ETypesRegistry) -> usize {
        self.inputs.len()
    }

    fn output_unchecked(
        &self,
        _registry: &ETypesRegistry,
        output: usize,
    ) -> miette::Result<OutputData> {
        Ok(OutputData {
            ty: self.inputs[output].clone().into(),
            name: output.to_string().into(),
        })
    }

    fn try_connect(
        &mut self,
        registry: &ETypesRegistry,
        commands: &mut SnarlCommands,
        from: &OutPin,
        to: &InPin,
        incoming_type: &NodePortType,
    ) -> miette::Result<()> {
        let Some(info) = incoming_type.item_info() else {
            return Ok(());
        };

        let i = to.id.input;
        if i == self.inputs.len() {
            self.inputs.push(info.clone());
        } else if self.inputs[i].ty() != incoming_type.ty() {
            self.inputs[i] = info.clone();
            // Reconnect the corresponding output pin to propagate type
            // changes and clear invalid connections
            commands.push(SnarlCommand::ReconnectOutput {
                id: OutPinId {
                    node: to.id.node,
                    output: i,
                },
            })
        }

        self._default_try_connect(registry, commands, from, to, incoming_type)?;
        Ok(())
    }

    fn try_disconnect(
        &mut self,
        _registry: &ETypesRegistry,
        commands: &mut SnarlCommands,
        from: &OutPin,
        to: &InPin,
    ) -> miette::Result<()> {
        commands.push(SnarlCommand::DisconnectRaw {
            from: from.id,
            to: to.id,
        });
        commands.push(SnarlCommand::DropOutputs {
            from: OutPinId {
                node: to.id.node,
                output: to.id.input,
            },
        });
        for i in to.id.input..(self.inputs.len() - 1) {
            self.inputs.as_mut_slice().swap(i, i + 1);
            commands.push(SnarlCommand::InputMovedRaw {
                from: InPinId {
                    node: to.id.node,
                    input: i + 1,
                },
                to: InPinId {
                    node: to.id.node,
                    input: i,
                },
            });
            commands.push(SnarlCommand::OutputMovedRaw {
                from: OutPinId {
                    node: to.id.node,
                    output: i + 1,
                },
                to: OutPinId {
                    node: to.id.node,
                    output: i,
                },
            });
        }

        self.inputs.pop();
        commands.push(SnarlCommand::DeletePinValue {
            pin: InPinId {
                node: to.id.node,
                input: self.inputs.len(),
            },
        });

        Ok(())
    }

    fn execute(
        &self,
        _registry: &ETypesRegistry,
        inputs: &[EValue],
        outputs: &mut Vec<EValue>,
    ) -> miette::Result<()> {
        for input in inputs.iter() {
            outputs.push(input.clone());
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct RerouteFactory;

impl NodeFactory for RerouteFactory {
    fn id(&self) -> Ustr {
        "reroute".into()
    }

    fn categories(&self) -> &'static [&'static str] {
        &["utility"]
    }

    fn create(&self) -> SnarlNode {
        Box::new(RerouteNode::default())
    }
}
