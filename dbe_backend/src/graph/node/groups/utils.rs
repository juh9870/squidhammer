use crate::etype::eitem::EItemInfo;
use crate::etype::EDataType;
use crate::graph::inputs::{GraphInput, GraphIoData, GraphOutput};
use crate::graph::node::commands::{SnarlCommand, SnarlCommands};
use crate::graph::node::ports::{InputData, NodePortType, OutputData};
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use egui_snarl::{InPinId, NodeId, OutPinId};
use itertools::Itertools;
use miette::bail;
use uuid::Uuid;

/// Gets the field at the given index, using the ID lookup table to account for
/// potential reordering
///
/// # Arguments
/// * `fields` - The fields to get the data from
/// * `ids` - local index-to-ID lookup table
/// * `index` - The index of the field to get
pub fn get_field<'ctx, IO: GraphIoData>(
    fields: &'ctx [IO],
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
    fields: &[IO],
    ids: &mut Vec<Uuid>,
    types: Option<&mut Vec<EDataType>>,
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

    commands.push(SnarlCommand::MarkDirty { node: node_id });

    *ids = new_fields;

    if let Some(types) = types {
        types.clear();
        types.extend(
            fields
                .iter()
                .map(|f| f.ty().unwrap_or_else(EDataType::null)),
        );
    }
}

pub fn map_group_inputs(
    registry: &ETypesRegistry,
    inputs: &[GraphInput],
    ids: &[Uuid],
    in_values: &[EValue],
    out_values: &mut Vec<EValue>,
) -> miette::Result<()> {
    out_values.clear();

    // Fill the outputs with the input values in the order of the IDs
    for (i, id) in ids.iter().enumerate() {
        let input_pos = if inputs.get(i).is_some_and(|f| f.id == *id) {
            i
        } else if let Some(idx) = ids.iter().position(|f| f == id) {
            idx
        } else {
            bail!("Input {} was deleted", id);
        };

        out_values.push(
            in_values
                .get(input_pos)
                .cloned()
                .or_else(|| {
                    inputs[input_pos]
                        .ty
                        .map(|ty| ty.default_value(registry).into_owned())
                })
                .unwrap_or_else(|| EValue::Null),
        );
    }

    Ok(())
}

pub fn map_group_outputs(
    registry: &ETypesRegistry,
    outputs: &[GraphOutput],
    ids: &[Uuid],
    in_values: &[EValue],
    out_values: &mut Vec<EValue>,
) -> miette::Result<()> {
    out_values.clear();

    // Fill the group outputs with the incoming values, matching the order of the IDs
    // New outputs will be filled with default values
    for (i, field) in outputs.iter().enumerate() {
        let Some(input_pos) = (if ids.get(i).is_some_and(|id| id == &field.id) {
            Some(i)
        } else {
            ids.iter().position(|f| f == &field.id)
        }) else {
            let default = field
                .ty
                .map(|f| f.default_value(registry).into_owned())
                .unwrap_or_else(|| EValue::Null);
            out_values.push(default);
            continue;
        };

        out_values.push(in_values[input_pos].clone());
    }

    // Check is any output was removed and incoming value now has no matching output
    for (i, id) in ids.iter().enumerate() {
        if outputs.get(i).is_some_and(|f| f.id == *id) {
            continue;
        }
        if outputs.iter().any(|f| f.id == *id) {
            continue;
        }

        bail!("Output {} was deleted", id);
    }

    Ok(())
}

pub fn get_port_input<IO: GraphIoData>(
    fields: &[IO],
    ids: &[Uuid],
    index: usize,
) -> miette::Result<InputData> {
    let Some(f) = get_field(fields, ids, index) else {
        return Ok(InputData {
            ty: NodePortType::Invalid,
            name: "!!unknown input!!".into(),
        });
    };

    Ok(InputData {
        ty: f
            .ty()
            .map(EItemInfo::simple_type)
            .map(NodePortType::Specific)
            .unwrap_or_else(|| NodePortType::BasedOnSource),
        name: f.name().into(),
    })
}

pub fn get_port_output<IO: GraphIoData>(
    fields: &[IO],
    ids: &[Uuid],
    index: usize,
) -> miette::Result<OutputData> {
    let Some(f) = get_field(fields, ids, index) else {
        return Ok(OutputData {
            ty: NodePortType::Invalid,
            name: "!!unknown output!!".into(),
        });
    };

    Ok(OutputData {
        ty: f
            .ty()
            .map(EItemInfo::simple_type)
            .map(NodePortType::Specific)
            .unwrap_or_else(|| NodePortType::BasedOnTarget),
        name: f.name().into(),
    })
}
