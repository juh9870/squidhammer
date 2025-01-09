use super::{
    FuncNode, FunctionalArgNames, FunctionalInputPortAdapter, FunctionalNode, FunctionalNodeOutput,
    FunctionalOutputPortAdapter, IntoFunctionalNode,
};
use crate::graph::node::ports::{InputData, OutputData};
use crate::graph::node::variables::ExecutionExtras;
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
        impl<$($in: FunctionalInputPortAdapter + 'static,)* O: FunctionalOutputPortAdapter + 'static, F: Fn($($in),*) -> O + Clone + Send + Sync + 'static> IntoFunctionalNode<($($in,)*), O> for F {
            type Fn = FuncNode<($($in,)*), O, F>;

            fn into_node(
                self,
                id: &'static str,
                input_names: <Self::Fn as FunctionalNode>::InputNames,
                output_names: <<Self::Fn as FunctionalNode>::Output as FunctionalNodeOutput>::OutputNames,
                categories: &'static[&'static str],
            ) -> Self::Fn {
                FuncNode {
                    f: self,
                    id,
                    input_names: input_names.as_ref(),
                    output_names: output_names.as_ref(),
                    marker1: Default::default(),
                    marker2: Default::default(),
                    categories,
                }
            }
        }
    };
}

macro_rules! get_edata_type {
    ($adapter:ty, $varname:ident, $($i:expr, $in:ident);*) => {
        paste::paste!{
            {
                $(const [< $in _IDX >]: usize = $i;)*
                match $varname {
                    $([< $in _IDX >] => <$in as $adapter>::port(),)*
                    _ => panic!("input index out of bounds"),
                }
            }
        }
    };
}

macro_rules! invoke_f {
    ($self:ident, $inputs:ident, $($i:expr, $in:ident);*) => {
        ($self.f)(
            $(
                $in::try_from(&$inputs[$i]).with_context(||format!("failed to convert input argument #{} {}", $i, $self.input_names[$i]))?,
            )*
        )
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
        impl<$($in: FunctionalInputPortAdapter + 'static,)* Output: FunctionalNodeOutput, F: Fn($($in),*) -> Output + Clone + Send + Sync> FunctionalNode for FuncNode<($($in,)*), Output, F> {
            type Output = Output;
            type InputNames = &'static [&'static str; count!($($in)*)];

            fn inputs_count(&self) -> usize {
                count!($($in)*)
            }

            #[allow(unused_variables)]
            fn input_unchecked(&self, input: usize) -> InputData {
                let port = enumerate!(get_edata_type(FunctionalInputPortAdapter, input), $($in)*);

                #[allow(unreachable_code)]
                InputData::new(port, self.input_names[input].into(),)
            }

            fn output_unchecked(&self, output: usize) -> OutputData {
                Output::output_unchecked(output, self.output_names)
            }

            #[allow(unused_variables)]
            fn execute(&self, inputs: &[EValue], outputs: &mut Vec<EValue>) -> miette::Result<()> {
                let result = enumerate!(invoke_f(self, inputs), $($in)*);

                FunctionalNodeOutput::write_results(result, outputs)
            }
        }

        impl<$($in: FunctionalInputPortAdapter + 'static,)* Output: FunctionalNodeOutput, F: Fn($($in),*) -> Output + Clone + Send + Sync> Node for FuncNode<($($in,)*), Output, F> {
            fn id(&self) -> Ustr {
                self.id.into()
            }

            fn inputs_count(&self, _context: NodeContext) -> usize {
                <Self as FunctionalNode>::inputs_count(self)
            }

            fn input_unchecked(&self, _context: NodeContext, input: usize) -> miette::Result<InputData> {
                Ok(<Self as FunctionalNode>::input_unchecked(self, input))
            }

            fn outputs_count(&self, _context: NodeContext) -> usize {
                <Self as FunctionalNode>::outputs_count(self)
            }

            fn output_unchecked(&self, _context: NodeContext, output: usize) -> miette::Result<OutputData> {
                Ok(<Self as FunctionalNode>::output_unchecked(self, output))
            }

            fn execute(&self, _context: NodeContext, inputs: &[EValue], outputs: &mut Vec<EValue>, _variables: &mut ExecutionExtras) -> miette::Result<ExecutionResult> {
                <Self as FunctionalNode>::execute(self, inputs, outputs)?;

                Ok(ExecutionResult::Done)
            }
        }
        impl<$($in: FunctionalInputPortAdapter + 'static,)* Output: FunctionalNodeOutput, F: Fn($($in),*) -> Output + Clone + Send + Sync> NodeFactory for FuncNode<($($in,)*), Output, F> {
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
            fn output_unchecked(output: usize, names: FunctionalArgNames) -> OutputData {
                let port = enumerate!(get_edata_type(FunctionalOutputPortAdapter, output), $($out)*);

                #[allow(unreachable_code)]
                OutputData::new(port,names[output].into())
            }

            fn write_results(self, outputs: &mut Vec<EValue>) -> miette::Result<()> {
                outputs.clear();

                paste::paste! {
                    let ($([< $out:lower >],)*) = self;

                    $(
                        outputs.push(Into::<EValue>::into([< $out:lower >]));
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
            impl_into_node!($([<I $i>]),*);
            impl_functional_node!($([<I $i>]),*);
            impl_functional_output!($([<O $i>]),*);
        }
    };
}

impl_all!();
impl_all!(1);
impl_all!(1, 2);
impl_all!(1, 2, 3);
