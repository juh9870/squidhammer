macro_rules! array_op_io {
    (@expand_item $self:ident @ref $kind:ident self . $($value:tt)*) => {
        $crate::graph::node::regional::array_ops::ArrayOpField::$kind(& ($self . $($value)*))
    };
    (@expand_item $self:ident @mut $kind:ident self . $($value:tt)*) => {
        $crate::graph::node::regional::array_ops::ArrayOpFieldMut::$kind(&mut ($self . $($value)*))
    };
    (@expand_item $self:ident @ref $kind:ident $value:expr) => {
        $crate::graph::node::regional::array_ops::ArrayOpField::$kind($value)
    };
    (@expand_item $self:ident @mut $kind:ident $value:expr) => {
        $crate::graph::node::regional::array_ops::ArrayOpFieldMut::$kind($value)
    };

    (@collection $self:ident @$mutability:tt $n:literal [$($kind:ident($($value:tt)*)),*]) => {
        return utils::smallvec_n![$n;
            $(
                $crate::graph::node::regional::array_ops::macros::array_op_io!(@expand_item $self @$mutability $kind $($value)*),
            )*
        ]
    };

    (@collection $self:ident @$mutability:tt [$($kind:ident($($value:tt)*)),*]) => {
        return [
            $(
                $crate::graph::node::regional::array_ops::macros::array_op_io!(@expand_item $self @$mutability $kind $($value)*),
            )*
        ]
    };

    ($io:ident { Start => [$($start_n:literal;)? $($start_kind:ident($($start_value:tt)*)),* $(,)?], End => [$($end_n:literal;)? $($end_kind:ident($($end_value:tt)*)),* $(,)?] }) => {
        fn $io(&self, kind: $crate::graph::node::regional::RegionIoKind) -> impl AsRef<[$crate::graph::node::regional::array_ops::ArrayOpField]> {
            match kind {
                $crate::graph::node::regional::RegionIoKind::Start => {
                    $crate::graph::node::regional::array_ops::macros::array_op_io!(
                        @collection self @ref $($start_n)? [$($start_kind($($start_value)*)),*]
                    );
                },
                $crate::graph::node::regional::RegionIoKind::End => {
                    $crate::graph::node::regional::array_ops::macros::array_op_io!(
                        @collection self @ref $($end_n)? [$($end_kind($($end_value)*)),*]
                    );
                }
            }
        }
        paste::paste! {
            fn [< $io _mut >](&mut self, kind: $crate::graph::node::regional::RegionIoKind) -> impl AsMut<[$crate::graph::node::regional::array_ops::ArrayOpFieldMut]> {
                match kind {
                    $crate::graph::node::regional::RegionIoKind::Start => {
                        $crate::graph::node::regional::array_ops::macros::array_op_io!(
                            @collection self @mut $($start_n)? [$($start_kind($($start_value)*)),*]
                        );
                    },
                    $crate::graph::node::regional::RegionIoKind::End => {
                        $crate::graph::node::regional::array_ops::macros::array_op_io!(
                            @collection self @mut $($end_n)? [$($end_kind($($end_value)*)),*]
                        );
                    }
                }
            }
        }
    };
}

pub(crate) use array_op_io;
