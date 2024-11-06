use crate::etype::default::DefaultEValue;
use crate::etype::EDataType;
use crate::json_utils::JsonValue;
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use atomic_refcell::AtomicRefCell;
use dyn_clone::DynClone;
use miette::bail;
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

#[derive(Debug, Copy, Clone)]
pub struct InputData {
    pub ty: EDataType,
    pub name: Ustr,
}

#[derive(Debug, Copy, Clone)]
pub struct OutputData {
    pub ty: EDataType,
    pub name: Ustr,
}

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
        let input = self.try_input(registry, input)?;
        Ok(input.ty.default_value(registry))
    }

    fn title(&self) -> String {
        self.id().to_string()
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
    ) -> miette::Result<InputData>;

    fn try_input(&self, registry: &ETypesRegistry, input: usize) -> miette::Result<InputData> {
        if input > self.outputs_count(registry) {
            bail!("input index out of bounds")
        } else {
            self.input_unchecked(registry, input)
        }
    }

    fn try_output(&self, registry: &ETypesRegistry, output: usize) -> miette::Result<InputData> {
        if output > self.outputs_count(registry) {
            bail!("output index out of bounds")
        } else {
            self.output_unchecked(registry, output)
        }
    }

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
