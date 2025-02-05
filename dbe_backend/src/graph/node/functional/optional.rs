use crate::etype::EDataType;
use crate::graph::node::functional::generic::GenericValue;
use crate::graph::node::functional::{functional_node, side_effects_node, C};
use crate::graph::node::NodeFactory;
use crate::value::ENumber;
use miette::miette;
use std::sync::Arc;

pub(super) fn nodes() -> Vec<Arc<dyn NodeFactory>> {
    vec![
        // unwrap node is a side effect node, since it can intentionally interrupt the execution
        side_effects_node(
            |_: C, value: Option<GenericValue<0>>, msg: String| {
                value.ok_or_else(|| {
                    let msg = msg.trim();
                    if msg.is_empty() {
                        miette!("value is None")
                    } else {
                        miette!("{}", msg)
                    }
                })
            },
            "unwrap",
            &["value", "message"],
            &["value"],
            &["optional"],
        ),
        functional_node(
            |ctx: C, value: Option<GenericValue<0>>| {
                value.unwrap_or_else(|| {
                    GenericValue(
                        ctx.input_types[0]
                            .unwrap_or_else(EDataType::null)
                            .default_value(ctx.context.registry)
                            .into_owned(),
                    )
                })
            },
            "unwrap_or_default",
            &["value"],
            &["value"],
            &["optional"],
        ),
        functional_node(
            |_: C, value: Option<GenericValue<0>>| ENumber::from(value.is_some()),
            "is_some",
            &["option"],
            &["is_some"],
            &["optional"],
        ),
        functional_node(
            |_: C, value: Option<GenericValue<0>>| ENumber::from(value.is_none()),
            "is_none",
            &["option"],
            &["is_none"],
            &["optional"],
        ),
    ]
}
