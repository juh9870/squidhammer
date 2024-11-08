use crate::etype::conversion::EDataTypeAdapter;
use crate::graph::node::{InputData, Node, NodeFactory, OutputData};
use crate::value::{ENumber, EValue};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::sync::Arc;

pub mod macros;

pub type FunctionalArgNames = &'static [&'static str];

pub trait FunctionalNode: Node {
    type Output: FunctionalNodeOutput;
    type InputNames: AsRef<[&'static str]> + 'static;
    fn inputs_count(&self) -> usize;
    fn outputs_count(&self) -> usize {
        Self::Output::outputs_count()
    }
    fn input_unchecked(&self, input: usize) -> InputData;
    fn output_unchecked(&self, output: usize) -> OutputData;
    fn execute(&self, inputs: &[EValue], outputs: &mut Vec<EValue>) -> miette::Result<()>;
}

pub trait FunctionalNodeOutput: 'static {
    type OutputNames: AsRef<[&'static str]> + 'static;
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
            categories: &[],
        }
    }
}

impl<Input, Output, F: Clone + Send + Sync> Debug for FuncNode<Input, Output, F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FuncNode")
            .field("id", &self.id)
            .field("input_names", &self.input_names)
            .field("output_names", &self.output_names)
            .finish()
    }
}

pub trait IntoFunctionalNode<Input, Output> {
    type Fn: FunctionalNode;
    fn into_node(
        self,
        id: &'static str,
        input_names: <Self::Fn as FunctionalNode>::InputNames,
        output_names: <<Self::Fn as FunctionalNode>::Output as FunctionalNodeOutput>::OutputNames,
        categories: &'static [&'static str],
    ) -> Self::Fn;
}

impl<T: EDataTypeAdapter + 'static> FunctionalNodeOutput for T {
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

pub fn functional_node<I, O, Arg: IntoFunctionalNode<I, O>>(
    arg: Arg,
    id: &'static str,
    input_names: <Arg::Fn as FunctionalNode>::InputNames,
    output_names: <<Arg::Fn as FunctionalNode>::Output as FunctionalNodeOutput>::OutputNames,
    categories: &'static [&'static str],
) -> Arc<Arg::Fn> {
    let node = arg.into_node(id, input_names, output_names, categories);

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
            |a: ENumber, b: ENumber| a + b,
            "add",
            &["a", "b"],
            &["sum"],
            &["math"],
        ),
        functional_node(
            |a: ENumber, b: ENumber| a - b,
            "subtract",
            &["a", "b"],
            &["difference"],
            &["math"],
        ),
        functional_node(
            |a: ENumber, b: ENumber| a * b,
            "multiply",
            &["a", "b"],
            &["product"],
            &["math"],
        ),
        functional_node(
            |a: ENumber, b: ENumber| a / b,
            "divide",
            &["a", "b"],
            &["quotient"],
            &["math"],
        ),
        functional_node(
            |a: ENumber, b: ENumber| ENumber::from(a.powf(b.0)),
            "power",
            &["a", "b"],
            &["result"],
            &["math"],
        ),
        functional_node(|a: ENumber| -a, "negate", &["a"], &["-a"], &["math"]),
        functional_node(
            |a: ENumber| ENumber::from(a.sqrt()),
            "square_root",
            &["a"],
            &["√a"],
            &["math"],
        ),
        functional_node(
            |a: ENumber| ENumber::from(a.abs()),
            "absolute",
            &["a"],
            &["|a|"],
            &["math"],
        ),
        functional_node(
            |a: ENumber| ENumber::from(a.floor()),
            "floor",
            &["a"],
            &["⌊a⌋"],
            &["math.rounding"],
        ),
        functional_node(
            |a: ENumber| ENumber::from(a.ceil()),
            "ceiling",
            &["a"],
            &["⌈a⌉"],
            &["math.rounding"],
        ),
        functional_node(
            |a: ENumber| ENumber::from(a.round()),
            "round",
            &["a"],
            &["⌊a⌋"],
            &["math.rounding"],
        ),
        functional_node(
            |a: ENumber| ENumber::from(a.trunc()),
            "truncate",
            &["a"],
            &["result"],
            &["math.rounding"],
        ),
        functional_node(
            |a: ENumber| ENumber::from(a.fract()),
            "fractional",
            &["a"],
            &["{a}"],
            &["math.rounding"],
        ),
        functional_node(
            |a: ENumber| ENumber::from(a.ln()),
            "natural_logarithm",
            &["a"],
            &["ln(a)"],
            &["math.transcendental"],
        ),
        functional_node(
            |a: ENumber| ENumber::from(a.log10()),
            "logarithm_base_10",
            &["a"],
            &["log10(a)"],
            &["math.transcendental"],
        ),
        functional_node(
            |a: ENumber| ENumber::from(a.sin()),
            "exponential",
            &["a"],
            &["e^a"],
            &["math.transcendental"],
        ),
        functional_node(
            |a: ENumber| ENumber::from(a.sin()),
            "sine",
            &["a"],
            &["sin(a)"],
            &["math.trigonometry"],
        ),
        functional_node(
            |a: ENumber| ENumber::from(a.cos()),
            "cosine",
            &["a"],
            &["cos(a)"],
            &["math.trigonometry"],
        ),
        functional_node(
            |a: ENumber| ENumber::from(a.tan()),
            "tangent",
            &["a"],
            &["tan(a)"],
            &["math.trigonometry"],
        ),
        functional_node(
            |a: ENumber| ENumber::from(a.asin()),
            "arc_sine",
            &["a"],
            &["asin(a)"],
            &["math.trigonometry"],
        ),
        functional_node(
            |a: ENumber| ENumber::from(a.acos()),
            "arc_cosine",
            &["a"],
            &["acos(a)"],
            &["math.trigonometry"],
        ),
        functional_node(
            |a: ENumber| ENumber::from(a.atan()),
            "arc_tangent",
            &["a"],
            &["atan(a)"],
            &["math.trigonometry"],
        ),
        functional_node(
            || ENumber::from(std::f64::consts::PI),
            "pi",
            &[],
            &["pi"],
            &["math.trigonometry", "math.constants"],
        ),
        functional_node(
            || ENumber::from(std::f64::consts::E),
            "e",
            &[],
            &["e"],
            &["math.transcendental", "math.constants"],
        ),
        functional_node(
            || ENumber::from(std::f64::consts::TAU),
            "tau",
            &[],
            &["tau"],
            &["math.trigonometry", "math.constants"],
        ),
        functional_node(
            || ENumber::from(1.618_033_988_749_895_f64),
            "golden_ratio",
            &[],
            &["phi"],
            &["math.constants"],
        ),
        functional_node(
            || ENumber::from(std::f64::consts::SQRT_2),
            "sqrt_2",
            &[],
            &["√2"],
            &["math.constants"],
        ),
    ]
}
