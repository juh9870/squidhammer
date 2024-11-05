use crate::etype::default::DefaultEValue;
use crate::etype::EDataType;
use crate::json_utils::JsonValue;
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use atomic_refcell::AtomicRefCell;
use dyn_clone::DynClone;
use miette::bail;
use std::borrow::Cow;
use std::fmt::Debug;
use std::sync::{Arc, LazyLock};
use ustr::{Ustr, UstrMap};

static NODE_FACTORIES: LazyLock<AtomicRefCell<UstrMap<Arc<dyn NodeFactory>>>> =
    LazyLock::new(|| AtomicRefCell::new(default_nodes().collect()));

fn default_nodes() -> impl Iterator<Item = (Ustr, Arc<dyn NodeFactory>)> {
    let v: Vec<Arc<dyn NodeFactory>> = vec![];
    v.into_iter().map(|item| (Ustr::from(&item.id()), item))
}

pub fn get_snarl_node(id: &Ustr) -> Option<SnarlNode> {
    NODE_FACTORIES.borrow().get(id).map(|f| f.create())
}

pub trait NodeFactory: Send + Sync + Debug + 'static {
    fn id(&self) -> Ustr;
    fn create(&self) -> SnarlNode;
}

pub type SnarlNode = Box<dyn Node>;

pub trait Node: DynClone + Debug + Send + Sync + 'static {
    /// Writes node state to json
    fn write_json(&self, _registry: &ETypesRegistry) -> miette::Result<JsonValue> {
        Ok(JsonValue::Null)
    }
    /// Loads node state from json
    fn parse_json(
        &mut self,
        _registry: &ETypesRegistry,
        _value: &mut JsonValue,
    ) -> miette::Result<()> {
        Ok(())
    }

    fn id(&self) -> Ustr;

    fn default_input_value(
        &self,
        registry: &ETypesRegistry,
        input: usize,
    ) -> miette::Result<DefaultEValue> {
        let inputs = self.inputs(registry)?;
        if input >= inputs.len() {
            bail!("Input index #{} out of bounds", input);
        }
        Ok(inputs[input].default_value(registry))
    }

    /// Node inputs
    fn inputs(&self, registry: &ETypesRegistry) -> miette::Result<Cow<'static, [EDataType]>>;

    /// Node outputs
    fn outputs(&self, registry: &ETypesRegistry) -> miette::Result<Cow<'static, [EDataType]>>;

    /// Whenever the node has side effects and must be executed
    fn has_side_effects(&self) -> bool {
        false
    }

    /// Execute the node
    fn execute(
        &self,
        registry: &ETypesRegistry,
        inputs: &[EValue],
        outputs: &mut Vec<EValue>,
    ) -> miette::Result<()>;
}
