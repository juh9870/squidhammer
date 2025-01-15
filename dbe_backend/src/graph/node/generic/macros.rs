macro_rules! generic_node_io {
    (@expand_item $self:ident @ref $kind:ident self . $($value:tt)*) => {
        $crate::graph::node::generic::GenericNodeField::$kind(& ($self . $($value)*))
    };
    (@expand_item $self:ident @mut $kind:ident self . $($value:tt)*) => {
        $crate::graph::node::generic::GenericNodeFieldMut::$kind(&mut ($self . $($value)*))
    };
    (@expand_item $self:ident @ref $kind:ident $value:expr) => {
        $crate::graph::node::generic::GenericNodeField::$kind($value)
    };
    (@expand_item $self:ident @mut $kind:ident $value:expr) => {
        $crate::graph::node::generic::GenericNodeFieldMut::$kind($value)
    };

    (@collection $self:ident @$mutability:tt $n:literal [$($kind:ident($($value:tt)*)),*]) => {
        return utils::smallvec_n![$n;
            $(
                $crate::graph::node::generic::macros::generic_node_io!(@expand_item $self @$mutability $kind $($value)*),
            )*
        ]
    };

    (@collection $self:ident @$mutability:tt [$($kind:ident($($value:tt)*)),*]) => {
        return [
            $(
                $crate::graph::node::generic::macros::generic_node_io!(@expand_item $self @$mutability $kind $($value)*),
            )*
        ]
    };

    ($io:ident { Start => [$($start_n:literal;)? $($start_kind:ident($($start_value:tt)*)),* $(,)?], End => [$($end_n:literal;)? $($end_kind:ident($($end_value:tt)*)),* $(,)?] }) => {
        fn $io(&self, kind: $crate::graph::node::regional::RegionIoKind) -> impl AsRef<[$crate::graph::node::generic::GenericNodeField]> {
            match kind {
                $crate::graph::node::regional::RegionIoKind::Start => {
                    $crate::graph::node::generic::macros::generic_node_io!(
                        @collection self @ref $($start_n)? [$($start_kind($($start_value)*)),*]
                    );
                },
                $crate::graph::node::regional::RegionIoKind::End => {
                    $crate::graph::node::generic::macros::generic_node_io!(
                        @collection self @ref $($end_n)? [$($end_kind($($end_value)*)),*]
                    );
                }
            }
        }
        paste::paste! {
            fn [< $io _mut >](&mut self, kind: $crate::graph::node::regional::RegionIoKind) -> impl AsMut<[$crate::graph::node::generic::GenericNodeFieldMut]> {
                match kind {
                    $crate::graph::node::regional::RegionIoKind::Start => {
                        $crate::graph::node::generic::macros::generic_node_io!(
                            @collection self @mut $($start_n)? [$($start_kind($($start_value)*)),*]
                        );
                    },
                    $crate::graph::node::regional::RegionIoKind::End => {
                        $crate::graph::node::generic::macros::generic_node_io!(
                            @collection self @mut $($end_n)? [$($end_kind($($end_value)*)),*]
                        );
                    }
                }
            }
        }
    };
    ($io:ident { [$($n:literal;)? $($kind:ident($($value:tt)*)),* $(,)?] }) => {
        fn $io(&self) -> impl AsRef<[$crate::graph::node::generic::GenericNodeField]> {
            $crate::graph::node::regional::RegionIoKind::Start => {
                $crate::graph::node::generic::macros::generic_node_io!(
                    @collection self @ref $($n)? [$($kind($($value)*)),*]
                );
            }
        }
        paste::paste! {
            fn [< $io _mut >](&mut self) -> impl AsMut<[$crate::graph::node::generic::GenericNodeFieldMut]> {
                $crate::graph::node::regional::RegionIoKind::Start => {
                    $crate::graph::node::generic::macros::generic_node_io!(
                        @collection self @mut $($n)? [$($kind($($value)*)),*]
                    );
                }
            }
        }
    };
}

pub(crate) use generic_node_io;
