use crate::graph::node::functional::values::AnyEValue;
use crate::graph::node::functional::{functional_node, side_effects_node, C};
use crate::graph::node::NodeFactory;
use crate::project::side_effects::SideEffect;
use miette::bail;
use std::sync::Arc;

pub(super) fn nodes() -> Vec<Arc<dyn NodeFactory>> {
    vec![
        side_effects_node(
            |ctx: C, key: AnyEValue, value: AnyEValue| {
                ctx.extras
                    .side_effects
                    .set_transient_storage(key.0, value.0)?;

                Ok(())
            },
            "set_local_storage_value",
            &["key", "value"],
            &[],
            &["utility.storage"],
        ),
        side_effects_node(
            |ctx: C, key: AnyEValue, value: AnyEValue| {
                ctx.extras.side_effects.push(SideEffect::SetGlobalStorage {
                    key: key.0,
                    value: Some(value.0),
                })?;

                Ok(())
            },
            "set_global_storage_value",
            &["key", "value"],
            &[],
            &["utility.storage"],
        ),
        functional_node(
            |ctx: C, key: AnyEValue| {
                let result = ctx.extras.side_effects.has_transient_storage(&key.0)?;

                Ok(result)
            },
            "has_storage_value",
            &["key"],
            &["result"],
            &["utility.storage"],
        ),
        functional_node(
            |ctx: C, key: AnyEValue| {
                let mappings = ctx.extras.side_effects.get_transient_storage(&key.0)?;

                Ok(mappings.map(|v| AnyEValue(v.clone())))
            },
            "try_get_storage_value",
            &["key"],
            &["value"],
            &["utility.storage"],
        ),
        functional_node(
            |ctx: C, key: AnyEValue| {
                let mappings = ctx.extras.side_effects.get_transient_storage(&key.0)?;

                let Some(mappings) = mappings else {
                    bail!("No storage value found for key {}", key.0);
                };

                Ok(AnyEValue(mappings.clone()))
            },
            "get_storage_value",
            &["key"],
            &["value"],
            &["utility.storage"],
        ),
    ]
}
