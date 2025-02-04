use crate::graph::node::functional::values::AnyEValue;
use crate::graph::node::functional::{functional_node, side_effects_node, C};
use crate::graph::node::NodeFactory;
use miette::bail;
use std::sync::Arc;

pub(super) fn nodes() -> Vec<Arc<dyn NodeFactory>> {
    vec![
        side_effects_node(
            |ctx: C, key: String, value: AnyEValue| {
                ctx.extras
                    .side_effects
                    .set_transient_storage(key, value.0)?;

                Ok(())
            },
            "set_transistent_value",
            &["key", "value"],
            &[],
            &["utility.storage"],
        ),
        functional_node(
            |ctx: C, key: String| {
                let result = ctx.extras.side_effects.has_transient_storage(&key)?;

                Ok(result)
            },
            "has_transistent_value",
            &["key"],
            &["result"],
            &["utility.storage"],
        ),
        functional_node(
            |ctx: C, key: String| {
                let mappings = ctx.extras.side_effects.get_transient_storage(&key)?;

                Ok(mappings.map(|v| AnyEValue(v.clone())))
            },
            "try_get_transistent_value",
            &["key"],
            &["value"],
            &["utility.storage"],
        ),
        functional_node(
            |ctx: C, key: String| {
                let mappings = ctx.extras.side_effects.get_transient_storage(&key)?;

                let Some(mappings) = mappings else {
                    bail!("No transistent value found for key {}", key);
                };

                Ok(AnyEValue(mappings.clone()))
            },
            "get_transistent_value",
            &["key"],
            &["value"],
            &["utility.storage"],
        ),
    ]
}
