use crate::editable::EditableFile;
use crate::value::draw::editor::EFieldEditor;
use crate::value::etype::registry::ETypesRegistry;
use egui_node_graph::{NodeId, UserResponseTrait};
use std::cell::RefCell;
use std::rc::Rc;

/// The response type is used to encode side-effects produced when drawing a
/// node in the graph. Most side-effects (creating new nodes, deleting existing
/// nodes, handling connections...) are already handled by the library, but this
/// mechanism allows creating additional side effects from user code.
#[derive(Clone, Debug)]
pub enum EditorGraphResponse {
    ChangeEditor {
        node_id: NodeId,
        field: String,
        editor: Box<dyn EFieldEditor>,
    },
}

impl UserResponseTrait for EditorGraphResponse {}

impl EditorGraphResponse {
    pub fn apply(self, state: &mut EditableFile, _registry: &Rc<RefCell<ETypesRegistry>>) {
        match self {
            EditorGraphResponse::ChangeEditor {
                editor,
                node_id,
                field,
            } => {
                let Some(node) = state.graph.graph.nodes.get_mut(node_id) else {
                    return;
                };
                node.user_data.editors.insert(field, editor);
            }
        }
    }
}
