use crate::workspace::graph::search::GraphSearch;
use dbe_backend::graph::node::ports::InputData;

pub fn node_for_input_pin(input: InputData) -> Option<GraphSearch> {
    if !input.ty.is_specific() {
        return None;
    }

    let search = GraphSearch::empty();

    Some(search)
}
