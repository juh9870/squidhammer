use super::generic::GenericFieldAdapter;
use super::{
    AsStaticSlice, FuncNode, FunctionalContext, FunctionalNode, FunctionalNodeOutput,
    IntoFunctionalNode,
};
use crate::etype::EDataType;
use crate::graph::node::extras::ExecutionExtras;
use crate::graph::node::generic::{GenericNodeField, GenericNodeFieldMut};
use crate::graph::node::NodeContext;
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use miette::Context;

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
        impl<$($in: GenericFieldAdapter + 'static,)* O: FunctionalNodeOutput + 'static, F: Fn(FunctionalContext, $($in),*) -> O + Clone + Send + Sync + 'static> IntoFunctionalNode<($($in,)*), O> for F {
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

// macro_rules! get_edata_type {
//     ($adapter:ty, $context:ident, $varname:ident, $($i:expr, $in:ident);*) => {
//         paste::paste!{
//             {
//                 $(const [< $in _IDX >]: usize = $i;)*
//                 match $varname {
//                     $([< $in _IDX >] => <$in as $adapter>::port($context),)*
//                     _ => panic!("input index out of bounds"),
//                 }
//             }
//         }
//     };
// }

macro_rules! get_field {
    ($method:ident, $registry:ident, $ty:ty, $varname:ident, $($i:expr, $in:ident);*) => {
        paste::paste!{
            {
                $(const [< $in _IDX >]: usize = $i;)*
                match $varname {
                    $([< $in _IDX >] => <$in as GenericFieldAdapter>::$method($registry, $ty),)*
                    _ => panic!("Index out of bounds: the length is {} but the index is {}", count!($($i)*), $varname),
                }
            }
        }
    };
}

macro_rules! invoke_f {
    ($self:ident, $ctx:ident, $input_types:ident, $inputs:ident, $($i:expr, $in:ident);*) => {
        {
            let registry = $ctx.context.registry;
            ($self.f)(
                $ctx,
                $(
                    $in::try_from_evalue(registry, $input_types[$i], &$inputs[$i]).with_context(||format!("failed to convert input argument #{} {}", $i, $self.input_names[$i]))?,
                )*
            )
        }
    };
}

macro_rules! write_results {
    ($context:ident, $output_types:ident, $outputs:ident, $($i:expr, $in:ident);*) => {
        $(
            $outputs.push($in.into_evalue($context.registry, $output_types[$i])?);
        )*
    };
}

macro_rules! impl_functional_node {
    ($($in:ident),*) => {
        impl_into_node!($($in),*);

        impl<$($in: GenericFieldAdapter + 'static,)* Output: FunctionalNodeOutput, F: Fn(FunctionalContext, $($in),*) -> Output + Clone + Send + Sync> FunctionalNode for FuncNode<($($in,)*), Output, F> {
            type Output = Output;
            type InputNames = &'static [&'static str; count!($($in)*)];

            fn id(&self) -> &'static str {
                self.id
            }

            fn input_names(&self) -> &[&str] {
                self.input_names
            }

            fn output_names(&self) -> &[&str] {
                self.output_names
            }

            #[allow(unused_variables)]
            fn input<'a>(registry: &ETypesRegistry, index: usize, ty: &'a Option<EDataType>) -> GenericNodeField<'a> {
                enumerate!(get_field(field, registry, ty, index), $($in)*)
            }

            #[allow(unused_variables)]
            fn input_mut<'a>(registry: &ETypesRegistry, index: usize, ty: &'a mut Option<EDataType>) -> GenericNodeFieldMut<'a> {
                enumerate!(get_field(field_mut, registry, ty, index), $($in)*)
            }
            fn input_generic_indices() -> impl IntoIterator<Item = Option<usize>> {
                [
                    $($in::type_index()),*
                ]
            }


            fn inputs_count() -> usize {
                count!($($in)*)
            }

            fn has_side_effects(&self) -> bool {
                self.has_side_effects
            }

            #[allow(unused_variables)]
            fn execute(
                &self,
                context: NodeContext,
                input_types: &[Option<EDataType>],
                output_types: &[Option<EDataType>],
                variables: &mut ExecutionExtras,
                inputs: &[EValue],
                outputs: &mut Vec<EValue>
            ) -> miette::Result<()> {
                let ctx = FunctionalContext::new(context, variables, input_types, output_types);
                let result = enumerate!(invoke_f(self, ctx, input_types, inputs), $($in)*);

                FunctionalNodeOutput::write_results(result, context, output_types, outputs)
            }

            fn categories(&self) -> &'static [&'static str] {
                self.categories
            }
        }
    };
}

macro_rules! impl_functional_output {
    ($($out:ident),*) => {
        impl<$($out: GenericFieldAdapter + 'static,)*> FunctionalNodeOutput for ($($out,)*) {
            type OutputNames = &'static [&'static str; count!($($out)*)];

            #[allow(unused_variables)]
            fn output<'a>(registry: &ETypesRegistry, index: usize, ty: &'a Option<EDataType>) -> GenericNodeField<'a> {
                enumerate!(get_field(field, registry, ty, index), $($out)*)
            }

            #[allow(unused_variables)]
            fn output_mut<'a>(registry: &ETypesRegistry, index: usize, ty: &'a mut Option<EDataType>) -> GenericNodeFieldMut<'a> {
                enumerate!(get_field(field_mut, registry, ty, index), $($out)*)
            }

            fn output_generic_indices() -> impl IntoIterator<Item = Option<usize>> {
                [
                    $($out::type_index()),*
                ]
            }

            fn outputs_count() -> usize {
                count!($($out)*)
            }

            #[allow(unused_variables)]
            fn write_results(
                self,
                context: NodeContext,
                output_types: &[Option<EDataType>],
                outputs: &mut Vec<EValue>,
            ) -> miette::Result<()> {
                outputs.clear();

                paste::paste! {
                    let ($([< $out:lower >],)*) = self;

                    enumerate!(write_results(context, output_types, outputs), $([< $out:lower >])*);

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
impl_all!(1, 2, 3, 4);
impl_all!(1, 2, 3, 4, 5);
