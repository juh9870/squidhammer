use super::{
    AsStaticSlice, FuncNode, FunctionalArgNames, FunctionalContext, FunctionalInputPortAdapter,
    FunctionalNode, FunctionalNodeOutput, FunctionalOutputPortAdapter, IntoFunctionalNode,
};
use crate::graph::node::extras::ExecutionExtras;
use crate::graph::node::ports::{InputData, OutputData};
use crate::graph::node::{ExecutionResult, Node, NodeContext, NodeFactory};
use crate::value::EValue;
use miette::Context;
use ustr::Ustr;

macro_rules! count {
    () => (0usize);
    ( $x:tt $($xs:tt)* ) => (1usize + count!($($xs)*));
}

macro_rules! enumerate {
    ($cb:ident($($args:tt),*), $($inputs:ident)*) => {
        enumerate!(@args[] $cb($($args),*), 0usize, $($inputs)*)
    };
    (@args[$($n:expr, $in:ident);*] $cb:ident($($args:tt),*), $i:expr, $curr:ident $($inputs:ident)*) => {
        enumerate!(@args[$($n, $in;)* $i, $curr] $cb($($args),*), $i + 1usize, $($inputs)*)
    };
    (@args[$($n:expr, $in:ident);*] $cb:ident($($args:tt),*), $i:expr, ) => {
        $cb!($($args),*, $($n, $in);*)
    };
}

macro_rules! impl_into_node {
    ($($in:ident),*) => {
        impl<$($in: FunctionalInputPortAdapter + 'static,)* O: FunctionalNodeOutput + 'static, F: Fn(FunctionalContext, $($in),*) -> O + Clone + Send + Sync + 'static> IntoFunctionalNode<($($in,)*), O> for F {
            type Fn = FuncNode<($($in,)*), O, F>;

            fn into_node(
                self,
                id: &'static str,
                input_names: <Self::Fn as FunctionalNode>::InputNames,
                output_names: <<Self::Fn as FunctionalNode>::Output as FunctionalNodeOutput>::OutputNames,
                categories: &'static[&'static str],
                has_side_effects: bool,
            ) -> Self::Fn {
                FuncNode {
                    f: self,
                    id,
                    input_names: input_names.as_static_slice(),
                    output_names: output_names.as_static_slice(),
                    marker1: Default::default(),
                    marker2: Default::default(),
                    categories,
                    has_side_effects,
                }
            }
        }
    };
}

macro_rules! get_edata_type {
    ($adapter:ty, $context:ident, $varname:ident, $($i:expr, $in:ident);*) => {
        paste::paste!{
            {
                $(const [< $in _IDX >]: usize = $i;)*
                match $varname {
                    $([< $in _IDX >] => <$in as $adapter>::port($context),)*
                    _ => panic!("input index out of bounds"),
                }
            }
        }
    };
}

macro_rules! invoke_f {
    ($self:ident, $ctx:ident, $inputs:ident, $($i:expr, $in:ident);*) => {
        {
            let registry = $ctx.0.registry;
            ($self.f)(
                $ctx,
                $(
                    $in::try_from_evalue(registry, &$inputs[$i]).with_context(||format!("failed to convert input argument #{} {}", $i, $self.input_names[$i]))?,
                )*
            )
        }
    };
}

// macro_rules! write_results {
//     ($self:ident, $outputs:ident, $($i:expr, $in:ident);*) => {
//         $(
//             $outputs.push(Into::<EValue>::into($self.$i));
//         )*
//     };
// }

macro_rules! impl_functional_node {
    ($($in:ident),*) => {
        impl_into_node!($($in),*);

        impl<$($in: FunctionalInputPortAdapter + 'static,)* Output: FunctionalNodeOutput, F: Fn(FunctionalContext, $($in),*) -> Output + Clone + Send + Sync> FunctionalNode for FuncNode<($($in,)*), Output, F> {
            type Output = Output;
            type InputNames = &'static [&'static str; count!($($in)*)];

            fn inputs_count(&self) -> usize {
                count!($($in)*)
            }

            #[allow(unused_variables)]
            fn input_unchecked(&self, context: NodeContext, input: usize) -> InputData {
                let port = enumerate!(get_edata_type(FunctionalInputPortAdapter, context, input), $($in)*);

                #[allow(unreachable_code)]
                InputData::new(port, self.input_names[input].into(),)
            }

            fn output_unchecked(&self, context: NodeContext, output: usize) -> OutputData {
                Output::output_unchecked(context, output, self.output_names)
            }

            #[allow(unused_variables)]
            fn execute(&self, context: NodeContext, variables: &mut ExecutionExtras, inputs: &[EValue], outputs: &mut Vec<EValue>) -> miette::Result<()> {
                let ctx = (context, variables);
                let result = enumerate!(invoke_f(self, ctx, inputs), $($in)*);

                FunctionalNodeOutput::write_results(result, context, outputs)
            }
        }

        impl<$($in: FunctionalInputPortAdapter + 'static,)* Output: FunctionalNodeOutput, F: Fn(FunctionalContext, $($in),*) -> Output + Clone + Send + Sync> Node for FuncNode<($($in,)*), Output, F> {
            fn id(&self) -> Ustr {
                self.id.into()
            }

            fn inputs_count(&self, _context: NodeContext) -> usize {
                <Self as FunctionalNode>::inputs_count(self)
            }

            fn input_unchecked(&self, context: NodeContext, input: usize) -> miette::Result<InputData> {
                Ok(<Self as FunctionalNode>::input_unchecked(self, context, input))
            }

            fn outputs_count(&self, _context: NodeContext) -> usize {
                <Self as FunctionalNode>::outputs_count(self)
            }

            fn output_unchecked(&self, context: NodeContext, output: usize) -> miette::Result<OutputData> {
                Ok(<Self as FunctionalNode>::output_unchecked(self, context, output))
            }

            fn has_side_effects(&self) -> bool {
                self.has_side_effects
            }

            fn execute(&self, context: NodeContext, inputs: &[EValue], outputs: &mut Vec<EValue>, variables: &mut ExecutionExtras) -> miette::Result<ExecutionResult> {
                <Self as FunctionalNode>::execute(self, context, variables, inputs, outputs)?;

                Ok(ExecutionResult::Done)
            }
        }
        impl<$($in: FunctionalInputPortAdapter + 'static,)* Output: FunctionalNodeOutput, F: Fn(FunctionalContext, $($in),*) -> Output + Clone + Send + Sync> NodeFactory for FuncNode<($($in,)*), Output, F> {
            fn id(&self) -> Ustr {
                self.id.into()
            }

            fn create(&self) -> Box<dyn Node> {
                Box::new(self.clone())
            }

            fn categories(&self) -> &'static [&'static str] {
                self.categories
            }
        }
    };
}

macro_rules! impl_functional_output {
    ($($out:ident),*) => {
        impl<$($out: FunctionalOutputPortAdapter + 'static,)*> FunctionalNodeOutput for ($($out,)*) {
            type OutputNames = &'static [&'static str; count!($($out)*)];

            fn outputs_count() -> usize {
                count!($($out)*)
            }

            #[allow(unused_variables)]
            fn output_unchecked(context: NodeContext, output: usize, names: FunctionalArgNames) -> OutputData {
                let port = enumerate!(get_edata_type(FunctionalOutputPortAdapter, context, output), $($out)*);

                #[allow(unreachable_code)]
                OutputData::new(port,names[output].into())
            }

            #[allow(unused_variables)]
            fn write_results(self, context: NodeContext, outputs: &mut Vec<EValue>) -> miette::Result<()> {
                outputs.clear();

                paste::paste! {
                    let ($([< $out:lower >],)*) = self;

                    $(
                        outputs.push([< $out:lower >].into_evalue(context.registry)?);
                    )*

                    Ok(())
                }
            }
        }
    };
}

macro_rules! impl_all {
    ($($i:tt),*) => {
        paste::paste!{
            impl_functional_node!($([<I $i>]),*);
            impl_functional_output!($([<O $i>]),*);
        }
    };
}

impl_all!();
impl_all!(1);
impl_all!(1, 2);
impl_all!(1, 2, 3);
