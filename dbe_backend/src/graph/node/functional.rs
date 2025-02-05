use crate::etype::default::DefaultEValue;
use crate::etype::EDataType;
use crate::graph::node::extras::ExecutionExtras;
use crate::graph::node::format_node::format_evalue_for_graph;
use crate::graph::node::functional::generic::GenericFieldAdapter;
use crate::graph::node::functional::values::AnyEValue;
use crate::graph::node::generic::{GenericNodeField, GenericNodeFieldMut};
use crate::graph::node::{NodeContext, NodeFactory};
use crate::registry::ETypesRegistry;
use crate::value::{ENumber, EValue};
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::sync::Arc;

mod generic;
mod impls;
mod macros;
mod values;

mod debug;
mod list;
mod mappings;
mod math;
mod optional;
mod raw_manip;
mod transient_storage;

pub type FunctionalArgNames = &'static [&'static str];

trait FunctionalNode: 'static + Clone + Debug + Hash + Send + Sync {
    type Output: FunctionalNodeOutput;
    type InputNames: AsStaticSlice;
    fn id(&self) -> &'static str;
    fn input_names(&self) -> &[&str];
    fn output_names(&self) -> &[&str];

    fn input<'a>(
        registry: &ETypesRegistry,
        index: usize,
        ty: &'a Option<EDataType>,
    ) -> GenericNodeField<'a>;
    fn input_mut<'a>(
        registry: &ETypesRegistry,
        index: usize,
        ty: &'a mut Option<EDataType>,
    ) -> GenericNodeFieldMut<'a>;
    fn inputs_count() -> usize;
    fn input_generic_indices() -> impl IntoIterator<Item = Option<usize>>;

    fn custom_default_value(
        &self,
        context: NodeContext,
        input: usize,
    ) -> miette::Result<Option<DefaultEValue>>;

    fn output<'a>(
        registry: &ETypesRegistry,
        index: usize,
        ty: &'a Option<EDataType>,
    ) -> GenericNodeField<'a> {
        Self::Output::output(registry, index, ty)
    }
    fn output_mut<'a>(
        registry: &ETypesRegistry,
        index: usize,
        ty: &'a mut Option<EDataType>,
    ) -> GenericNodeFieldMut<'a> {
        Self::Output::output_mut(registry, index, ty)
    }
    fn outputs_count() -> usize {
        Self::Output::outputs_count()
    }
    fn output_generic_indices() -> impl IntoIterator<Item = Option<usize>> {
        Self::Output::output_generic_indices()
    }

    fn has_side_effects(&self) -> bool;
    fn execute(
        &self,
        context: NodeContext,
        input_types: &[Option<EDataType>],
        output_types: &[Option<EDataType>],
        variables: &mut ExecutionExtras,
        inputs: &[EValue],
        outputs: &mut Vec<EValue>,
    ) -> miette::Result<()>;

    fn categories(&self) -> &'static [&'static str];
}

trait FunctionalNodeOutput: 'static {
    type OutputNames: AsStaticSlice;
    fn output<'a>(
        registry: &ETypesRegistry,
        index: usize,
        ty: &'a Option<EDataType>,
    ) -> GenericNodeField<'a>;
    fn output_mut<'a>(
        registry: &ETypesRegistry,
        index: usize,
        ty: &'a mut Option<EDataType>,
    ) -> GenericNodeFieldMut<'a>;
    fn outputs_count() -> usize;
    fn output_generic_indices() -> impl IntoIterator<Item = Option<usize>>;
    fn write_results(
        self,
        context: NodeContext,
        output_types: &[Option<EDataType>],
        outputs: &mut Vec<EValue>,
    ) -> miette::Result<()>;
}

pub struct FuncNode<Input, Output, F: Clone + Send + Sync + 'static> {
    f: F,
    id: &'static str,
    input_names: FunctionalArgNames,
    output_names: FunctionalArgNames,
    marker1: PhantomData<fn() -> Input>,
    marker2: PhantomData<fn() -> Output>,
    categories: &'static [&'static str],
    has_side_effects: bool,
}

impl<Input, Output, F: Clone + Send + Sync> Clone for FuncNode<Input, Output, F> {
    fn clone(&self) -> Self {
        Self {
            f: self.f.clone(),
            id: self.id,
            input_names: self.input_names,
            output_names: self.output_names,
            marker1: Default::default(),
            marker2: Default::default(),
            categories: self.categories,
            has_side_effects: self.has_side_effects,
        }
    }
}
impl<Input, Output, F: Clone + Send + Sync> Hash for FuncNode<Input, Output, F> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state); // no state to hash, id is enough
    }
}

impl<Input, Output, F: Clone + Send + Sync> Debug for FuncNode<Input, Output, F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FuncNode")
            .field("id", &self.id)
            .field("input_names", &self.input_names)
            .field("output_names", &self.output_names)
            .field("categories", &self.categories)
            .field("has_side_effects", &self.has_side_effects)
            .finish()
    }
}

trait IntoFunctionalNode<Input, Output> {
    type Fn: FunctionalNode;
    fn into_node(
        self,
        id: &'static str,
        input_names: <Self::Fn as FunctionalNode>::InputNames,
        output_names: <<Self::Fn as FunctionalNode>::Output as FunctionalNodeOutput>::OutputNames,
        categories: &'static [&'static str],
        has_side_effects: bool,
    ) -> Self::Fn;
}

impl<T: GenericFieldAdapter + 'static> FunctionalNodeOutput for T {
    type OutputNames = <(T,) as FunctionalNodeOutput>::OutputNames;

    fn output<'a>(
        registry: &ETypesRegistry,
        index: usize,
        ty: &'a Option<EDataType>,
    ) -> GenericNodeField<'a> {
        <(T,) as FunctionalNodeOutput>::output(registry, index, ty)
    }

    fn output_mut<'a>(
        registry: &ETypesRegistry,
        index: usize,
        ty: &'a mut Option<EDataType>,
    ) -> GenericNodeFieldMut<'a> {
        <(T,) as FunctionalNodeOutput>::output_mut(registry, index, ty)
    }

    fn outputs_count() -> usize {
        <(T,) as FunctionalNodeOutput>::outputs_count()
    }

    fn output_generic_indices() -> impl IntoIterator<Item = Option<usize>> {
        <(T,) as FunctionalNodeOutput>::output_generic_indices()
    }

    fn write_results(
        self,
        context: NodeContext,
        output_types: &[Option<EDataType>],
        outputs: &mut Vec<EValue>,
    ) -> miette::Result<()> {
        <(T,) as FunctionalNodeOutput>::write_results((self,), context, output_types, outputs)
    }
}

impl<T: FunctionalNodeOutput> FunctionalNodeOutput for miette::Result<T> {
    type OutputNames = T::OutputNames;

    fn output<'a>(
        registry: &ETypesRegistry,
        index: usize,
        ty: &'a Option<EDataType>,
    ) -> GenericNodeField<'a> {
        T::output(registry, index, ty)
    }

    fn output_mut<'a>(
        registry: &ETypesRegistry,
        index: usize,
        ty: &'a mut Option<EDataType>,
    ) -> GenericNodeFieldMut<'a> {
        T::output_mut(registry, index, ty)
    }

    fn outputs_count() -> usize {
        T::outputs_count()
    }

    fn output_generic_indices() -> impl IntoIterator<Item = Option<usize>> {
        T::output_generic_indices()
    }

    fn write_results(
        self,
        context: NodeContext,
        output_types: &[Option<EDataType>],
        outputs: &mut Vec<EValue>,
    ) -> miette::Result<()> {
        let result = self?;
        FunctionalNodeOutput::write_results(result, context, output_types, outputs)
    }
}

trait AsStaticSlice {
    fn as_static_slice(&self) -> &'static [&'static str];
}

impl<const N: usize> AsStaticSlice for &'static [&'static str; N] {
    fn as_static_slice(&self) -> &'static [&'static str] {
        (*self).as_slice()
    }
}

struct FunctionalContext<'this, 'ctx, 'extras> {
    context: NodeContext<'ctx>,
    extras: &'this mut ExecutionExtras<'extras>,
    #[allow(dead_code)]
    input_types: &'this [Option<EDataType>],
    #[allow(dead_code)]
    output_types: &'this [Option<EDataType>],
}

impl<'this, 'ctx, 'extras> FunctionalContext<'this, 'ctx, 'extras> {
    pub fn new(
        context: NodeContext<'ctx>,
        extras: &'this mut ExecutionExtras<'extras>,
        input_types: &'this [Option<EDataType>],
        output_types: &'this [Option<EDataType>],
    ) -> Self {
        Self {
            context,
            extras,
            input_types,
            output_types,
        }
    }
}

type C<'this, 'ctx, 'extras> = FunctionalContext<'this, 'ctx, 'extras>;

fn side_effects_node<I, O, Arg: IntoFunctionalNode<I, O>>(
    arg: Arg,
    id: &'static str,
    input_names: <Arg::Fn as FunctionalNode>::InputNames,
    output_names: <<Arg::Fn as FunctionalNode>::Output as FunctionalNodeOutput>::OutputNames,
    categories: &'static [&'static str],
) -> Arc<Arg::Fn> {
    let node = arg.into_node(id, input_names, output_names, categories, true);
    Arc::new(node)
}

fn functional_node<I, O, Arg: IntoFunctionalNode<I, O>>(
    arg: Arg,
    id: &'static str,
    input_names: <Arg::Fn as FunctionalNode>::InputNames,
    output_names: <<Arg::Fn as FunctionalNode>::Output as FunctionalNodeOutput>::OutputNames,
    categories: &'static [&'static str],
) -> Arc<Arg::Fn> {
    let node = arg.into_node(id, input_names, output_names, categories, false);

    // assert_eq!(
    //     FunctionalNode::inputs_count(&node),
    //     input_names.len(),
    //     "input count mismatch: function has {} inputs, but {} names were provided",
    //     FunctionalNode::inputs_count(&node),
    //     input_names.len()
    // );
    // assert_eq!(
    //     FunctionalNode::outputs_count(&node),
    //     output_names.len(),
    //     "output count mismatch: function has {} outputs, but {} names were provided",
    //     FunctionalNode::outputs_count(&node),
    //     output_names.len()
    // );

    Arc::new(node)
}

pub fn functional_nodes() -> Vec<Arc<dyn NodeFactory>> {
    let mut nodes: Vec<Arc<dyn NodeFactory>> = vec![
        functional_node(
            |_: C, a: bool, b: bool| a == b,
            "bool_equals",
            &["a", "b"],
            &["a == b"],
            &["boolean"],
        ),
        functional_node(
            |_: C, a: bool, b: bool| a != b,
            "bool_not_equals",
            &["a", "b"],
            &["a != b"],
            &["boolean"],
        ),
        functional_node(
            |_: C, a: bool| !a,
            "bool_invert",
            &["a"],
            &["not a"],
            &["boolean"],
        ),
        functional_node(
            |_: C, a: bool, b: bool| a && b,
            "bool_and",
            &["a", "b"],
            &["a and b"],
            &["boolean"],
        ),
        functional_node(
            |_: C, a: bool, b: bool| a || b,
            "bool_or",
            &["a", "b"],
            &["a or b"],
            &["boolean"],
        ),
        functional_node(
            |_: C, a: String, b: String| a + &b,
            "concat",
            &["a", "b"],
            &["result"],
            &["string"],
        ),
        functional_node(
            |_: C, a: String| a.to_lowercase(),
            "lower_case",
            &["a"],
            &["result"],
            &["string"],
        ),
        functional_node(
            |_: C, a: String| a.to_uppercase(),
            "upper_case",
            &["a"],
            &["result"],
            &["string"],
        ),
        functional_node(
            |_: C, a: String| ENumber::from(a.len() as f64),
            "string_length",
            &["a"],
            &["length"],
            &["string"],
        ),
        functional_node(
            |_: C, a: ENumber| a,
            "numeric_value",
            &["a"],
            &["a"],
            &["value", "math"],
        ),
        functional_node(
            |_: C, a: bool| a,
            "boolean_value",
            &["a"],
            &["a"],
            &["value", "boolean"],
        ),
        functional_node(
            |_: C, a: String| a,
            "string_value",
            &["a"],
            &["a"],
            &["value", "string"],
        ),
        functional_node(
            |_: C, value: AnyEValue| format_evalue_for_graph(&value.0),
            "to_string",
            &["value"],
            &["result"],
            &["string"],
        ),
    ];

    nodes.extend(debug::nodes());
    nodes.extend(list::nodes());
    nodes.extend(mappings::nodes());
    nodes.extend(math::nodes());
    nodes.extend(optional::nodes());
    nodes.extend(raw_manip::nodes());
    nodes.extend(transient_storage::nodes());

    nodes
}
