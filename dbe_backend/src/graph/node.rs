use crate::etype::default::DefaultEValue;
use crate::graph::node::commands::{SnarlCommand, SnarlCommands};
use crate::graph::node::functional::functional_nodes;
use crate::graph::node::reroute::RerouteFactory;
use crate::json_utils::JsonValue;
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use atomic_refcell::{AtomicRef, AtomicRefCell};
use downcast_rs::{impl_downcast, Downcast};
use dyn_clone::DynClone;
use egui_snarl::{InPin, OutPin};
use miette::bail;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::sync::{Arc, LazyLock};
use ustr::{Ustr, UstrMap};

pub mod commands;
pub mod enum_node;
pub mod functional;
pub mod list;
pub mod ports;
pub mod reroute;
pub mod saving_node;
pub mod struct_node;

static NODE_FACTORIES: LazyLock<AtomicRefCell<UstrMap<Arc<dyn NodeFactory>>>> =
    LazyLock::new(|| AtomicRefCell::new(default_nodes().collect()));

type FactoriesByCategory = BTreeMap<&'static str, Vec<Arc<dyn NodeFactory>>>;
static NODE_FACTORIES_BY_CATEGORY: LazyLock<AtomicRefCell<FactoriesByCategory>> =
    LazyLock::new(|| {
        AtomicRefCell::new({
            let mut map: BTreeMap<&str, Vec<Arc<dyn NodeFactory>>> = BTreeMap::new();
            for (_, fac) in default_nodes() {
                for cat in fac.categories() {
                    map.entry(*cat).or_default().push(fac.clone());
                }
            }
            map
        })
    });

fn default_nodes() -> impl Iterator<Item = (Ustr, Arc<dyn NodeFactory>)> {
    let mut v: Vec<Arc<dyn NodeFactory>> = functional_nodes();
    v.push(Arc::new(RerouteFactory));
    v.push(Arc::new(StructNodeFactory));
    v.push(Arc::new(EnumNodeFactory));
    v.push(Arc::new(SavingNodeFactory));
    v.push(Arc::new(ListNodeFactory));
    v.into_iter().map(|item| (Ustr::from(&item.id()), item))
}

pub fn get_snarl_node(id: &Ustr) -> Option<SnarlNode> {
    NODE_FACTORIES.borrow().get(id).map(|f| f.create())
}

pub fn all_node_factories() -> AtomicRef<'static, UstrMap<Arc<dyn NodeFactory>>> {
    NODE_FACTORIES.borrow()
}

pub fn node_factories_by_category() -> AtomicRef<'static, FactoriesByCategory> {
    NODE_FACTORIES_BY_CATEGORY.borrow()
}

pub trait NodeFactory: Send + Sync + Debug + 'static {
    fn id(&self) -> Ustr;
    fn categories(&self) -> &'static [&'static str];
    fn create(&self) -> SnarlNode;
}

pub type SnarlNode = Box<dyn Node>;

pub trait Node: DynClone + Debug + Send + Sync + Downcast + 'static {
    /// Writes node state to json
    fn write_json(&self, registry: &ETypesRegistry) -> miette::Result<JsonValue> {
        let _ = (registry,);
        Ok(JsonValue::Null)
    }
    /// Loads node state from json
    fn parse_json(
        &mut self,
        registry: &ETypesRegistry,
        value: &mut JsonValue,
    ) -> miette::Result<()> {
        let _ = (registry, value);
        Ok(())
    }

    fn id(&self) -> Ustr;

    fn default_input_value(
        &self,
        registry: &ETypesRegistry,
        input: usize,
    ) -> miette::Result<DefaultEValue> {
        let input = self.try_input(registry, input)?;
        Ok(input.ty.default_value(registry))
    }

    fn title(&self, registry: &ETypesRegistry) -> String {
        let _ = (registry,);
        self.id().to_string()
    }

    /// Determines if the node has inline editable values
    fn has_inline_values(&self) -> miette::Result<bool> {
        Ok(true)
    }

    /// Node inputs
    fn inputs_count(&self, registry: &ETypesRegistry) -> usize;

    /// Returns the type of the input pin
    /// # Panics
    /// This method panics if the input index is out of bounds
    fn input_unchecked(&self, registry: &ETypesRegistry, input: usize)
        -> miette::Result<InputData>;

    /// Node outputs
    fn outputs_count(&self, registry: &ETypesRegistry) -> usize;

    /// Returns the type of the output pin
    /// # Panics
    /// This method panics if the input index is out of bounds
    fn output_unchecked(
        &self,
        registry: &ETypesRegistry,
        output: usize,
    ) -> miette::Result<OutputData>;

    fn try_input(&self, registry: &ETypesRegistry, input: usize) -> miette::Result<InputData> {
        let count = self.inputs_count(registry);
        if input >= count {
            bail!("input index {} out of bounds (max {})", input, count - 1)
        } else {
            self.input_unchecked(registry, input)
        }
    }

    fn try_output(&self, registry: &ETypesRegistry, output: usize) -> miette::Result<OutputData> {
        let count = self.outputs_count(registry);
        if output >= count {
            bail!("output index {} out of bounds (max {})", output, count - 1)
        } else {
            self.output_unchecked(registry, output)
        }
    }

    /// Attempts to create a connection to the input pin of the node
    /// Returns true if the connection can be made
    ///
    /// On success, the connection may or may not be made depending on the node logic
    ///
    /// Nodes may mutate their internal state when a connection is made
    fn try_connect(
        &mut self,
        registry: &ETypesRegistry,
        commands: &mut SnarlCommands,
        from: &OutPin,
        to: &InPin,
        incoming_type: &NodePortType,
    ) -> miette::Result<()> {
        self._default_try_connect(registry, commands, from, to, incoming_type)?;
        Ok(())
    }

    /// Disconnect the input pin of the node
    ///
    /// On success, the provided connection should no longer exist after executing emitted commands
    fn try_disconnect(
        &mut self,
        registry: &ETypesRegistry,
        commands: &mut SnarlCommands,
        from: &OutPin,
        to: &InPin,
    ) -> miette::Result<()> {
        self._default_try_disconnect(registry, commands, from, to)
    }

    /// Whenever the node has side effects and must be executed
    fn has_side_effects(&self) -> bool {
        false
    }

    /// Execute the node
    fn execute_side_effects(
        &self,
        registry: &ETypesRegistry,
        inputs: &[EValue],
        outputs: &mut Vec<EValue>,
        side_effects: SideEffectsContext<'_>,
    ) -> miette::Result<()> {
        let _ = (registry, inputs, outputs, side_effects);
        panic!("Node has no side effects")
    }

    /// Execute the node
    fn execute(
        &self,
        registry: &ETypesRegistry,
        inputs: &[EValue],
        outputs: &mut Vec<EValue>,
    ) -> miette::Result<()>;

    fn _default_try_connect(
        &mut self,
        registry: &ETypesRegistry,
        commands: &mut SnarlCommands,
        from: &OutPin,
        to: &InPin,
        incoming_type: &NodePortType,
    ) -> miette::Result<bool> {
        let ty = self.try_input(registry, to.id.input)?;
        if NodePortType::compatible(registry, incoming_type, &ty.ty) {
            // TODO: support for multi-connect ports
            if !to.remotes.is_empty() {
                commands.push(SnarlCommand::DropInputsRaw { to: to.id });
            }

            commands.push(SnarlCommand::ConnectRaw {
                from: from.id,
                to: to.id,
            });

            return Ok(true);
        }

        Ok(false)
    }

    fn _default_try_disconnect(
        &mut self,
        registry: &ETypesRegistry,
        commands: &mut SnarlCommands,
        from: &OutPin,
        to: &InPin,
    ) -> miette::Result<()> {
        let _ = (registry,);
        commands.push(SnarlCommand::DisconnectRaw {
            from: from.id,
            to: to.id,
        });
        Ok(())
    }
}

impl_downcast!(Node);

/// Implements write_json and parse_json for the node by serializing whole node struct via serde
macro_rules! impl_serde_node {
    () => {
        fn write_json(
            &self,
            _registry: &ETypesRegistry,
        ) -> miette::Result<$crate::json_utils::JsonValue> {
            miette::IntoDiagnostic::into_diagnostic(serde_json::value::to_value(&self))
        }

        fn parse_json(
            &mut self,
            _registry: &ETypesRegistry,
            value: &mut $crate::json_utils::JsonValue,
        ) -> miette::Result<()> {
            miette::IntoDiagnostic::into_diagnostic(Self::deserialize(value.take()))
                .map(|node| *self = node)
        }
    };
}

use crate::graph::node::enum_node::EnumNodeFactory;
use crate::graph::node::list::ListNodeFactory;
use crate::graph::node::ports::{InputData, NodePortType, OutputData};
use crate::graph::node::saving_node::SavingNodeFactory;
use crate::graph::node::struct_node::StructNodeFactory;
use crate::project::side_effects::SideEffectsContext;
pub(crate) use impl_serde_node;
