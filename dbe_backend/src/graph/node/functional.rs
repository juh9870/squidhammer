use crate::etype::conversion::EItemInfoAdapter;
use crate::graph::node::format_node::format_evalue_for_graph;
use crate::graph::node::ports::NodePortType;
use crate::graph::node::variables::ExecutionExtras;
use crate::graph::node::{InputData, Node, NodeContext, NodeFactory, OutputData};
use crate::project::side_effects::SideEffect;
use crate::value::{ENumber, EValue};
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::sync::Arc;

pub mod macros;

pub type FunctionalArgNames = &'static [&'static str];

trait FunctionalNode: Node {
    type Output: FunctionalNodeOutput;
    type InputNames: AsStaticSlice;
    fn inputs_count(&self) -> usize;
    fn outputs_count(&self) -> usize {
        Self::Output::outputs_count()
    }
    fn input_unchecked(&self, input: usize) -> InputData;
    fn output_unchecked(&self, output: usize) -> OutputData;
    fn execute(
        &self,
        context: NodeContext,
        variables: &mut ExecutionExtras,
        inputs: &[EValue],
        outputs: &mut Vec<EValue>,
    ) -> miette::Result<()>;
}

trait FunctionalNodeOutput: 'static {
    type OutputNames: AsStaticSlice;
    fn outputs_count() -> usize;
    fn output_unchecked(output: usize, names: FunctionalArgNames) -> OutputData;
    fn write_results(self, outputs: &mut Vec<EValue>) -> miette::Result<()>;
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

impl<T: FunctionalOutputPortAdapter + 'static> FunctionalNodeOutput for T {
    type OutputNames = <(T,) as FunctionalNodeOutput>::OutputNames;

    fn outputs_count() -> usize {
        <(T,) as FunctionalNodeOutput>::outputs_count()
    }

    fn output_unchecked(output: usize, names: FunctionalArgNames) -> OutputData {
        <(T,) as FunctionalNodeOutput>::output_unchecked(output, names)
    }

    fn write_results(self, outputs: &mut Vec<EValue>) -> miette::Result<()> {
        <(T,) as FunctionalNodeOutput>::write_results((self,), outputs)
    }
}

impl<T: FunctionalNodeOutput> FunctionalNodeOutput for miette::Result<T> {
    type OutputNames = T::OutputNames;

    fn outputs_count() -> usize {
        T::outputs_count()
    }

    fn output_unchecked(output: usize, names: FunctionalArgNames) -> OutputData {
        T::output_unchecked(output, names)
    }

    fn write_results(self, outputs: &mut Vec<EValue>) -> miette::Result<()> {
        let result = self?;
        FunctionalNodeOutput::write_results(result, outputs)
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

pub trait FunctionalInputPortAdapter:
    Into<EValue> + for<'a> TryFrom<&'a EValue, Error = miette::Report>
{
    fn port() -> NodePortType;
}

pub trait FunctionalOutputPortAdapter:
    Into<EValue> + for<'a> TryFrom<&'a EValue, Error = miette::Report>
{
    fn port() -> NodePortType;
}

impl<T: EItemInfoAdapter> FunctionalInputPortAdapter for T {
    fn port() -> NodePortType {
        T::edata_type().into()
    }
}

impl<T: EItemInfoAdapter> FunctionalOutputPortAdapter for T {
    fn port() -> NodePortType {
        T::edata_type().into()
    }
}

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

impl FunctionalInputPortAdapter for AnyEValue {
    fn port() -> NodePortType {
        NodePortType::BasedOnSource
    }
}

type FunctionalContext<'this, 'ctx, 'extras> =
    (NodeContext<'ctx>, &'this mut ExecutionExtras<'extras>);

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
        side_effects_node(
            |ctx: C, value: AnyEValue| {
                // ignore errors
                let _ = ctx
                    .1
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
