/// Implements write_json and parse_json for the node by serializing whole node struct via serde
macro_rules! impl_serde_node {
    () => {
        fn write_json(
            &self,
            _registry: &$crate::registry::ETypesRegistry,
        ) -> miette::Result<$crate::json_utils::JsonValue> {
            miette::IntoDiagnostic::into_diagnostic(serde_json::value::to_value(&self))
        }

        fn parse_json(
            &mut self,
            _registry: &$crate::registry::ETypesRegistry,
            value: &mut $crate::json_utils::JsonValue,
        ) -> miette::Result<()> {
            miette::IntoDiagnostic::into_diagnostic(Self::deserialize(value.take()))
                .map(|node| *self = node)
        }
    };
}

pub(super) use impl_serde_node;
