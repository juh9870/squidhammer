use crate::graph::inputs::GraphIoData;
use crate::graph::node::commands::{SnarlCommand, SnarlCommands};
use egui_snarl::{InPinId, NodeId, OutPinId};
use itertools::Itertools;
use smallvec::SmallVec;
use uuid::Uuid;

/// Gets the field at the given index, using the ID lookup table to account for
/// potential reordering
///
/// # Arguments
/// * `fields` - The fields to get the data from
/// * `ids` - local index-to-ID lookup table
/// * `index` - The index of the field to get
pub fn get_field<'ctx, IO: GraphIoData>(
    fields: &'ctx SmallVec<[IO; 1]>,
    ids: &[Uuid],
    index: usize,
) -> Option<&'ctx IO> {
    // If the index is not in the fields, return the output at the same index directly, it's likely a new output
    let Some(id) = ids.get(index) else {
        return fields.get(index);
    };

    // Check the output at the same position
    if let Some(output) = fields.get(index) {
        if output.id() == id {
            return Some(output);
        }
    }

    // In case the field was moved, find it by ID
    fields.iter().find(|o| o.id() == id)
}

/// Synchronizes the inputs or outputs of a node and updates the ID lookup
/// table to match the new order and presence of fields
///
/// # Arguments
/// * `commands` - Command buffer
/// * `fields` - The new fields to synchronize with
/// * `ids` - local index-to-ID lookup table
/// * `node_id` - The ID of the node
pub fn sync_fields<IO: GraphIoData>(
    commands: &mut SnarlCommands,
    fields: &SmallVec<[IO; 1]>,
    ids: &mut Vec<Uuid>,
    node_id: NodeId,
) {
    if ids.len() == fields.len()
        && ids
            .iter()
            .zip_eq(fields.iter())
            .all(|(id, input)| id == input.id())
    {
        return;
    }

    let new_fields = fields.iter().map(|i| *i.id()).collect_vec();

    let mut moves = vec![];
    let mut drops = vec![];

    for (i, id) in ids.iter().enumerate() {
        if let Some(pos) = new_fields.iter().position(|i| i == id) {
            if pos != i {
                moves.push((i, pos));
            }
        } else {
            drops.push(i);
        }
    }

    for drop_pos in drops {
        if IO::is_input() {
            commands.push(SnarlCommand::DropOutputs {
                from: OutPinId {
                    output: drop_pos,
                    node: node_id,
                },
            })
        } else {
            commands.push(SnarlCommand::DropInputs {
                to: InPinId {
                    input: drop_pos,
                    node: node_id,
                },
            })
        }
    }

    for (from, to) in moves {
        if IO::is_input() {
            commands.push(SnarlCommand::OutputMovedRaw {
                from: OutPinId {
                    output: from,
                    node: node_id,
                },
                to: OutPinId {
                    output: to,
                    node: node_id,
                },
            })
        } else {
            commands.push(SnarlCommand::InputMovedRaw {
                from: InPinId {
                    input: from,
                    node: node_id,
                },
                to: InPinId {
                    input: to,
                    node: node_id,
                },
            })
        }
    }

    *ids = new_fields;
}
