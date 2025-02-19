use crate::graph::node::functional::generic::GenericValue;
use crate::graph::node::functional::{functional_node, C};
use crate::graph::node::NodeFactory;
use crate::value::ENumber;
use miette::bail;
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
            |_: C, list: Vec<GenericValue<0>>, index: ENumber| {
                let mut idx = index.0 as isize;
                if idx < 0 {
                    idx += list.len() as isize;
                }
                if idx < 0 || idx as usize >= list.len() {
                    bail!(
                        "Index out of bound: the list has {} elements, but the index is {}",
                        list.len(),
                        index.0 as usize
                    );
                }
                Ok(list[idx as usize].clone())
            },
            "list_get",
            &["list", "index"],
            &["item"],
            &["list"],
        ),
        functional_node(
            |_: C, list: Vec<GenericValue<0>>, index: ENumber| {
                let mut idx = index.0 as isize;
                if idx < 0 {
                    idx += list.len() as isize;
                }
                if idx < 0 || idx as usize >= list.len() {
                    Ok(None)
                } else {
                    Ok(Some(list[idx as usize].clone()))
                }
            },
            "list_try_get",
            &["list", "index"],
            &["item"],
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
            |_: C, mut list: Vec<GenericValue<0>>, a: ENumber, b: ENumber| {
                list.swap(a.0 as usize, b.0 as usize);
                list
            },
            "list_swap",
            &["list", "first", "second"],
            &["list"],
            &["list"],
        ),
        functional_node(
            |_: C, mut list: Vec<GenericValue<0>>, a: ENumber, b: ENumber| {
                // moves item from position a to b
                let item = list.remove(a.0 as usize);
                if b.0 as usize == list.len() {
                    list.push(item);
                } else {
                    list.insert(b.0 as usize, item);
                }
                list
            },
            "list_move",
            &["list", "from", "to"],
            &["list"],
            &["list"],
        ),
        functional_node(
            |_: C, mut list: Vec<GenericValue<0>>, amount: ENumber| {
                list.rotate_left(amount.0 as usize);
                list
            },
            "list_rotate_left",
            &["list", "amount"],
            &["list"],
            &["list"],
        ),
        functional_node(
            |_: C, mut list: Vec<GenericValue<0>>, amount: ENumber| {
                list.rotate_right(amount.0 as usize);
                list
            },
            "list_rotate_right",
            &["list", "amount"],
            &["list"],
            &["list"],
        ),
        functional_node(
            |_: C, mut list: Vec<GenericValue<0>>| {
                list.reverse();
                list
            },
            "list_reverse",
            &["list"],
            &["list"],
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
