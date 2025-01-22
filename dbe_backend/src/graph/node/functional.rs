use crate::etype::conversion::EItemInfoAdapter;
use crate::etype::eitem::EItemInfo;
use crate::etype::EDataType;
use crate::graph::node::extras::ExecutionExtras;
use crate::graph::node::format_node::format_evalue_for_graph;
use crate::graph::node::functional::generic::{GenericFieldAdapter, GenericValue};
use crate::graph::node::generic::{GenericNodeField, GenericNodeFieldMut};
use crate::graph::node::{NodeContext, NodeFactory};
use crate::project::side_effects::SideEffect;
use crate::registry::ETypesRegistry;
use crate::value::{ENumber, EValue};
use miette::{bail, miette};
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::sync::Arc;
use ustr::ustr;

pub mod generic;
pub mod impls;
pub mod macros;

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

// pub trait FunctionalInputPortAdapter: ValueAdapter {
//     fn port(context: NodeContext) -> NodePortType;
// }
//
// pub trait FunctionalOutputPortAdapter: ValueAdapter {
//     fn port(context: NodeContext) -> NodePortType;
// }
//
// impl<T: EItemInfoAdapter> FunctionalInputPortAdapter for T {
//     fn port(context: NodeContext) -> NodePortType {
//         T::edata_type(context.registry).into()
//     }
// }
//
// impl<T: EItemInfoAdapter> FunctionalOutputPortAdapter for T {
//     fn port(context: NodeContext) -> NodePortType {
//         T::edata_type(context.registry).into()
//     }
// }

struct AnyEValue(EValue);

impl TryFrom<&EValue> for AnyEValue {
    type Error = miette::Report;

    fn try_from(value: &EValue) -> Result<Self, Self::Error> {
        Ok(Self(value.clone()))
    }
}

impl From<AnyEValue> for EValue {
    fn from(value: AnyEValue) -> Self {
        value.0
    }
}

impl EItemInfoAdapter for AnyEValue {
    fn edata_type(_registry: &ETypesRegistry) -> EItemInfo {
        EItemInfo::simple_type(EDataType::Unknown)
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
    vec![
        functional_node(
            |_: C, a: ENumber, b: ENumber| a + b,
            "add",
            &["a", "b"],
            &["sum"],
            &["math"],
        ),
        functional_node(
            |_: C, a: ENumber, b: ENumber| a - b,
            "subtract",
            &["a", "b"],
            &["difference"],
            &["math"],
        ),
        functional_node(
            |_: C, a: ENumber, b: ENumber| a * b,
            "multiply",
            &["a", "b"],
            &["product"],
            &["math"],
        ),
        functional_node(
            |_: C, a: ENumber, b: ENumber| a / b,
            "divide",
            &["a", "b"],
            &["quotient"],
            &["math"],
        ),
        functional_node(
            |_: C, a: ENumber, b: ENumber| ENumber::from(a.powf(b.0)),
            "power",
            &["a", "b"],
            &["result"],
            &["math"],
        ),
        functional_node(|_: C, a: ENumber| -a, "negate", &["a"], &["-a"], &["math"]),
        functional_node(
            |_: C, a: ENumber| ENumber::from(a.sqrt()),
            "square_root",
            &["a"],
            &["√a"],
            &["math"],
        ),
        functional_node(
            |_: C, a: ENumber| ENumber::from(a.abs()),
            "absolute",
            &["a"],
            &["|a|"],
            &["math"],
        ),
        functional_node(
            |_: C, a: ENumber| ENumber::from(a.floor()),
            "floor",
            &["a"],
            &["⌊a⌋"],
            &["math.rounding"],
        ),
        functional_node(
            |_: C, a: ENumber| ENumber::from(a.ceil()),
            "ceiling",
            &["a"],
            &["⌈a⌉"],
            &["math.rounding"],
        ),
        functional_node(
            |_: C, a: ENumber| ENumber::from(a.round()),
            "round",
            &["a"],
            &["⌊a⌋"],
            &["math.rounding"],
        ),
        functional_node(
            |_: C, a: ENumber| ENumber::from(a.trunc()),
            "truncate",
            &["a"],
            &["result"],
            &["math.rounding"],
        ),
        functional_node(
            |_: C, a: ENumber| ENumber::from(a.fract()),
            "fractional",
            &["a"],
            &["{a}"],
            &["math.rounding"],
        ),
        functional_node(
            |_: C, a: ENumber| ENumber::from(a.ln()),
            "natural_logarithm",
            &["a"],
            &["ln(a)"],
            &["math.transcendental"],
        ),
        functional_node(
            |_: C, a: ENumber| ENumber::from(a.log10()),
            "logarithm_base_10",
            &["a"],
            &["log10(a)"],
            &["math.transcendental"],
        ),
        functional_node(
            |_: C, a: ENumber| ENumber::from(a.sin()),
            "exponential",
            &["a"],
            &["e^a"],
            &["math.transcendental"],
        ),
        functional_node(
            |_: C, a: ENumber| ENumber::from(a.sin()),
            "sine",
            &["a"],
            &["sin(a)"],
            &["math.trigonometry"],
        ),
        functional_node(
            |_: C, a: ENumber| ENumber::from(a.cos()),
            "cosine",
            &["a"],
            &["cos(a)"],
            &["math.trigonometry"],
        ),
        functional_node(
            |_: C, a: ENumber| ENumber::from(a.tan()),
            "tangent",
            &["a"],
            &["tan(a)"],
            &["math.trigonometry"],
        ),
        functional_node(
            |_: C, a: ENumber| ENumber::from(a.asin()),
            "arc_sine",
            &["a"],
            &["asin(a)"],
            &["math.trigonometry"],
        ),
        functional_node(
            |_: C, a: ENumber| ENumber::from(a.acos()),
            "arc_cosine",
            &["a"],
            &["acos(a)"],
            &["math.trigonometry"],
        ),
        functional_node(
            |_: C, a: ENumber| ENumber::from(a.atan()),
            "arc_tangent",
            &["a"],
            &["atan(a)"],
            &["math.trigonometry"],
        ),
        functional_node(
            |_: C| ENumber::from(std::f64::consts::PI),
            "pi",
            &[],
            &["pi"],
            &["math.trigonometry", "math.constants"],
        ),
        functional_node(
            |_: C| ENumber::from(std::f64::consts::E),
            "e",
            &[],
            &["e"],
            &["math.transcendental", "math.constants"],
        ),
        functional_node(
            |_: C| ENumber::from(std::f64::consts::TAU),
            "tau",
            &[],
            &["tau"],
            &["math.trigonometry", "math.constants"],
        ),
        functional_node(
            |_: C| ENumber::from(1.618_033_988_749_895_f64),
            "golden_ratio",
            &[],
            &["phi"],
            &["math.constants"],
        ),
        functional_node(
            |_: C| ENumber::from(std::f64::consts::SQRT_2),
            "sqrt_2",
            &[],
            &["√2"],
            &["math.constants"],
        ),
        functional_node(
            |_: C, a: ENumber, b: ENumber| a > b,
            "greater_than",
            &["a", "b"],
            &["a > b"],
            &["math.comparison"],
        ),
        functional_node(
            |_: C, a: ENumber, b: ENumber| a >= b,
            "greater_or_equal_than",
            &["a", "b"],
            &["a >= b"],
            &["math.comparison"],
        ),
        functional_node(
            |_: C, a: ENumber, b: ENumber| a < b,
            "lesser_than",
            &["a", "b"],
            &["a < b"],
            &["math.comparison"],
        ),
        functional_node(
            |_: C, a: ENumber, b: ENumber| a <= b,
            "lesser_or_equal_than",
            &["a", "b"],
            &["a <= b"],
            &["math.comparison"],
        ),
        functional_node(
            |_: C, a: ENumber, b: ENumber| a == b,
            "num_equals",
            &["a", "b"],
            &["a == b"],
            &["math.comparison"],
        ),
        functional_node(
            |_: C, a: bool, b: bool| a == b,
            "bool_equals",
            &["a", "b"],
            &["a == b"],
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
            |_: C, a: bool, b: bool| a != b,
            "bool_not_equals",
            &["a", "b"],
            &["a != b"],
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
        functional_node(
            |_: C, value: Option<GenericValue<0>>| value.ok_or_else(|| miette!("value is None")),
            "unwrap",
            &["value"],
            &["value"],
            &["optional"],
        ),
        functional_node(
            |_: C, value: AnyEValue, field: String| {
                let value = value.0;
                fn get_value(value: &EValue, field: &str) -> miette::Result<Option<EValue>> {
                    match value {
                        EValue::Struct { fields, .. } => Ok(fields.get(&ustr(field)).cloned()),
                        EValue::Enum { data, .. } => get_value(data, field),
                        _ => bail!("value is not a struct or an enum"),
                    }
                }
                let value = get_value(&value, &field)?;
                Ok(value.map(AnyEValue))
            },
            "get_field",
            &["value", "field"],
            &["result"],
            &["optional.raw"],
        ),
        side_effects_node(
            |ctx: C, value: AnyEValue| {
                // ignore errors
                let _ = ctx
                    .extras
                    .side_effects
                    .push(SideEffect::ShowDebug { value: value.0 });
            },
            "debug_print",
            &["value"],
            &[],
            &["debug"],
        ),
    ]
}
