use crate::etype::econst::ETypeConst;
use crate::etype::eitem::EItemInfo;
use crate::etype::EDataType;
use crate::graph::node::commands::{SnarlCommand, SnarlCommands};
use crate::graph::node::ports::NodePortType;
use crate::graph::node::{impl_serde_node, InputData, Node, NodeFactory, OutputData, SnarlNode};
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use egui_snarl::{InPin, InPinId, OutPin, OutPinId};
use serde::{Deserialize, Serialize};
use ustr::Ustr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListNode {
    item: EDataType,
    /// Determines whenever the list retains its type once empty
    ///
    /// This flag is NOT persisted
    #[serde(skip)]
    fixed: bool,
    #[serde(skip)]
    items_count: usize,
}

impl ListNode {
    pub fn new() -> Self {
        Self {
            item: EDataType::Const {
                value: ETypeConst::Null,
            },
            fixed: false,
            items_count: 0,
        }
    }

    pub fn of_type(ty: EDataType) -> Self {
        Self {
            item: ty,
            fixed: true,
            items_count: 0,
        }
    }
}

impl Default for ListNode {
    fn default() -> Self {
        Self::new()
    }
}

impl Node for ListNode {
    impl_serde_node!();

    fn id(&self) -> Ustr {
        ListNodeFactory.id()
    }

    fn has_inline_values(&self) -> miette::Result<bool> {
        Ok(false)
    }

    fn inputs_count(&self, _registry: &ETypesRegistry) -> usize {
        self.items_count + 1
    }

    fn input_unchecked(
        &self,
        _registry: &ETypesRegistry,
        input: usize,
    ) -> miette::Result<InputData> {
        Ok(InputData {
            ty: if self.items_count == 0 && !self.fixed {
                NodePortType::Any
            } else {
                EItemInfo::simple_type(self.item).into()
            },
            name: if input == self.items_count {
                "+".into()
            } else {
                input.to_string().into()
            },
        })
    }

    fn outputs_count(&self, _registry: &ETypesRegistry) -> usize {
        1
    }

    fn output_unchecked(
        &self,
        registry: &ETypesRegistry,
        output: usize,
    ) -> miette::Result<OutputData> {
        Ok(OutputData {
            ty: EItemInfo::simple_type(registry.list_of(self.item)).into(),
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
        if self.items_count == 0 && self.item != incoming_type.ty() {
            self.item = incoming_type.ty();
            commands.push(SnarlCommand::ReconnectOutput {
                id: OutPinId {
                    node: to.id.node,
                    output: 0,
                },
            })
        }

        if self._default_try_connect(registry, commands, from, to, incoming_type)? {
            self.items_count += 1;
        }

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
        for i in to.id.input..(self.items_count - 1) {
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
        }
        self.items_count -= 1;
        commands.push(SnarlCommand::DeletePinValue {
            pin: InPinId {
                node: to.id.node,
                input: self.items_count,
            },
        });

        Ok(())
    }

    fn execute(
        &self,
        registry: &ETypesRegistry,
        inputs: &[EValue],
        outputs: &mut Vec<EValue>,
    ) -> miette::Result<()> {
        let mut values = vec![];
        // TODO: check for inputs count to match items_count
        for input in inputs.iter().take(self.items_count).cloned() {
            values.push(input);
        }
        outputs.clear();
        outputs.push(EValue::List {
            id: registry.list_id_of(self.item),
            values,
        });
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ListNodeFactory;

impl NodeFactory for ListNodeFactory {
    fn id(&self) -> Ustr {
        "list".into()
    }

    fn categories(&self) -> &'static [&'static str] {
        &["list"]
    }

    fn create(&self) -> SnarlNode {
        Box::new(ListNode::default())
    }
}
