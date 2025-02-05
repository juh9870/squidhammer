use crate::graph::node::functional::values::AnyEValue;
use crate::graph::node::functional::{side_effects_node, C};
use crate::graph::node::NodeFactory;
use crate::project::side_effects::SideEffect;
use miette::bail;
use std::sync::Arc;

pub(super) fn nodes() -> Vec<Arc<dyn NodeFactory>> {
    vec![
        side_effects_node(
            |ctx: C, value: AnyEValue| {
                // ignore errors
                let _ = ctx
                    .extras
                    .side_effects
                    .push(SideEffect::ShowDebug { value: value.0 });
            },
            "debug_print",
            &["value"],
            &[],
            &["debug"],
        ),
        side_effects_node(
            |_: C, value: bool, msg: String| {
                if !value {
                    let msg = msg.trim();
                    if msg.is_empty() {
                        bail!("assert failed")
                    } else {
                        bail!("{}", msg)
                    }
                }
                Ok(())
            },
            "assert",
            &["value", "message"],
            &[],
            &["debug"],
        ),
        side_effects_node(
            |_: C, value: bool, msg: String| {
                if value {
                    let msg = msg.trim();
                    if msg.is_empty() {
                        bail!("assert_not failed")
                    } else {
                        bail!("{}", msg)
                    }
                }
                Ok(())
            },
            "assert_not",
            &["value", "message"],
            &[],
            &["debug"],
        ),
        side_effects_node(
            |_: C, a: AnyEValue, b: AnyEValue, msg: String| {
                if a != b {
                    let msg = msg.trim();
                    if msg.is_empty() {
                        bail!("assert_equals failed: {} != {}", a.0, b.0)
                    } else {
                        bail!(help = msg, "assert_equals failed: {} != {}", a.0, b.0)
                    }
                }
                Ok(())
            },
            "assert_equals",
            &["a", "b", "message"],
            &[],
            &["debug"],
        ),
        side_effects_node(
            |_: C, a: AnyEValue, b: AnyEValue, msg: String| {
                if a == b {
                    let msg = msg.trim();
                    if msg.is_empty() {
                        bail!("assert_not_equals failed: {} == {}", a.0, b.0)
                    } else {
                        bail!(help = msg, "assert_not_equals failed: {} == {}", a.0, b.0)
                    }
                }
                Ok(())
            },
            "assert_not_equals",
            &["a", "b", "message"],
            &[],
            &["debug"],
        ),
    ]
}
