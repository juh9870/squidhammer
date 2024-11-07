use crate::etype::conversion::EDataTypeAdapter;
use crate::graph::node::{InputData, Node, OutputData};
use crate::value::EValue;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

pub mod macros;

pub type FunctionalArgNames = &'static [&'static str];

pub trait FunctionalNode: Node {
    type Output: FunctionalNodeOutput;
    fn inputs_count(&self) -> usize;
    fn outputs_count(&self) -> usize {
        Self::Output::outputs_count()
    }
    fn input_unchecked(&self, input: usize) -> InputData;
    fn output_unchecked(&self, output: usize) -> OutputData;
    fn execute(&self, inputs: &[EValue], outputs: &mut Vec<EValue>) -> miette::Result<()>;
}

pub trait FunctionalNodeOutput: 'static {
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
        input_names: FunctionalArgNames,
        output_names: FunctionalArgNames,
    ) -> Self::Fn;
}

impl<T: EDataTypeAdapter + 'static> FunctionalNodeOutput for T {
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
    input_names: FunctionalArgNames,
    output_names: FunctionalArgNames,
) -> Arg::Fn {
    let node = arg.into_node(id, input_names, output_names);

    assert_eq!(
        FunctionalNode::inputs_count(&node),
        input_names.len(),
        "input count mismatch: function has {} inputs, but {} names were provided",
        FunctionalNode::inputs_count(&node),
        input_names.len()
    );
    assert_eq!(
        FunctionalNode::outputs_count(&node),
        output_names.len(),
        "output count mismatch: function has {} outputs, but {} names were provided",
        FunctionalNode::outputs_count(&node),
        output_names.len()
    );

    node
}
