use crate::graph::node::functional::generic::GenericValue;
use crate::graph::node::functional::{functional_node, C};
use crate::graph::node::NodeFactory;
use crate::value::ENumber;
use std::sync::Arc;

pub(super) fn nodes() -> Vec<Arc<dyn NodeFactory>> {
    vec![
        functional_node(
            |_: C, value: Vec<GenericValue<0>>| ENumber::from(value.len() as f64),
            "list_length",
            &["list"],
            &["length"],
            &["list"],
        ),
        functional_node(
            |_: C, mut list: Vec<GenericValue<0>>, item: GenericValue<0>| {
                list.push(item);
                list
            },
            "list_push",
            &["list", "item"],
            &["list"],
            &["list"],
        ),
        functional_node(
            |_: C, mut a: Vec<GenericValue<0>>, b: Vec<GenericValue<0>>| {
                a.extend(b);
                a
            },
            "list_concat",
            &["a", "b"],
            &["result"],
            &["list"],
        ),
        functional_node(
            |_: C, mut list: Vec<GenericValue<0>>, index: ENumber, item: GenericValue<0>| {
                list.insert(index.0 as usize, item);
                list
            },
            "list_insert",
            &["list", "index", "item"],
            &["list"],
            &["list"],
        ),
        functional_node(
            |_: C, mut list: Vec<GenericValue<0>>| {
                let item = list.pop();
                (item, list)
            },
            "list_pop",
            &["list"],
            &["item", "list"],
            &["list"],
        ),
        functional_node(
            |_: C, mut list: Vec<GenericValue<0>>, index: ENumber| {
                let item = list.remove(index.0 as usize);
                (item, list)
            },
            "list_remove",
            &["list", "index"],
            &["item", "list"],
            &["list"],
        ),
        functional_node(
            |_: C, list: Vec<GenericValue<0>>, item: GenericValue<0>| list.contains(&item),
            "list_contains",
            &["list", "item"],
            &["contains"],
            &["list"],
        ),
    ]
}
