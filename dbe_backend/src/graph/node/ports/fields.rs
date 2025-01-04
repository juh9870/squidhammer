use crate::graph::node::commands::{RearrangeIndices, SnarlCommand, SnarlCommands};
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use egui_snarl::{InPinId, NodeId, OutPinId};
use itertools::Itertools;
use miette::bail;
use std::fmt::Display;
use strum::EnumIs;

pub mod mappers;

/// Trait for mapping fields to local representation and matching them
pub trait FieldMapper {
    /// The field type
    type Field;
    /// The locally-stored representation fo the field
    type Local: PartialEq + Display;
    type Type;

    /// Checks if the field matches the local representation
    fn matches(&self, field: &Self::Field, local: &Self::Local) -> bool;

    /// Converts the field to the local representation
    fn to_local(&self, field: &Self::Field) -> Self::Local;

    /// Gets field type
    fn field_type(&self, field: &Self::Field) -> Self::Type {
        let _ = (field,);
        unimplemented!()
    }

    fn default_value(&self, field: &Self::Field, registry: &ETypesRegistry) -> EValue {
        let _ = (field, registry);
        unimplemented!()
    }
}

/// Gets the field at the given index, using the local cached fields to account
/// for potential reordering
///
/// # Arguments
/// * `fields` - The fields to get the data from
/// * `local` - local cached fields representation
/// * `index` - The index of the field to get
/// * `matcher` - Equality function to compare the field with the local representation
pub fn get_field<'ctx, Mapper: FieldMapper>(
    mapper: &Mapper,
    fields: &'ctx [Mapper::Field],
    local: &[Mapper::Local],
    index: usize,
) -> Option<&'ctx Mapper::Field> {
    // If the index is not in the fields, return the field at the same index directly, it's likely a new field
    let Some(id) = local.get(index) else {
        return fields.get(index);
    };

    // Check the output at the same position
    if let Some(output) = fields.get(index) {
        if mapper.matches(output, id) {
            return Some(output);
        }
    }

    // In case the field was moved, find it by ID
    fields.iter().find(|o| mapper.matches(o, id))
}

/// Same as [sync_fields_and_types] but without types
pub fn sync_fields<Mapper: FieldMapper>(
    mapper: &Mapper,
    commands: &mut SnarlCommands,
    fields: &[Mapper::Field],
    ids: &mut Vec<Mapper::Local>,
    node_id: NodeId,
    io: IoDirection,
) {
    sync_fields_and_types(mapper, commands, fields, ids, None, node_id, io)
}

/// Synchronizes the inputs or outputs of a node and updates the local cached
/// fields representation to match the new order and presence of fields,
/// optionally updating the types as well
///
/// # Arguments
/// * `commands` - Command buffer
/// * `fields` - The new fields to synchronize with
/// * `ids` - local index-to-ID lookup table
/// * `types` - Optional types to synchronize with fields
/// * `node_id` - The ID of the node
/// * `io` - The direction of the IO to synchronize
/// * `matcher` - Equality function to compare the field with the local representation
/// * `to_local` - Function to convert the field to the local representation
/// * `to_type` - Function to convert the field to the type
pub fn sync_fields_and_types<Mapper: FieldMapper>(
    mapper: &Mapper,
    commands: &mut SnarlCommands,
    fields: &[Mapper::Field],
    ids: &mut Vec<Mapper::Local>,
    types: Option<&mut Vec<Mapper::Type>>,
    node_id: NodeId,
    io: IoDirection,
) {
    if ids.len() == fields.len()
        && ids
            .iter()
            .zip_eq(fields.iter())
            .all(|(id, field)| mapper.matches(field, id))
    {
        return;
    }

    let new_fields = fields.iter().map(|f| mapper.to_local(f)).collect_vec();

    let mut rearrangements = None::<RearrangeIndices>;
    let mut drops = vec![];

    for (i, id) in ids.iter().enumerate() {
        if let Some(pos) = new_fields.iter().position(|i| i == id) {
            if pos != i {
                let indices = rearrangements
                    .get_or_insert_with(|| (0..ids.len()).collect::<RearrangeIndices>());
                indices[i] = pos;
            }
        } else {
            drops.push(i);
        }
    }

    for drop_pos in drops {
        if io.is_output() {
            commands.push(SnarlCommand::DropOutputs {
                from: OutPinId {
                    output: drop_pos,
                    node: node_id,
                },
            })
        } else {
            commands.push(SnarlCommand::DeletePinValue {
                pin: InPinId {
                    input: drop_pos,
                    node: node_id,
                },
            });
            commands.push(SnarlCommand::DropInputs {
                to: InPinId {
                    input: drop_pos,
                    node: node_id,
                },
            })
        }
    }

    if let Some(rearrangements) = rearrangements {
        if io.is_output() {
            commands.push(SnarlCommand::OutputsRearrangedRaw {
                node: node_id,
                indices: rearrangements,
            })
        } else {
            commands.push(SnarlCommand::InputsRearrangedRaw {
                node: node_id,
                indices: rearrangements,
            })
        }
    }

    commands.push(SnarlCommand::MarkDirty { node: node_id });

    *ids = new_fields;

    if let Some(types) = types {
        types.clear();
        types.extend(fields.iter().map(|f| mapper.field_type(f)));
    }
}

/// Maps the incoming values to the local fields representation, filling the
/// missing inputs with default values
pub fn map_inputs<Mapper: FieldMapper>(
    mapper: &Mapper,
    registry: &ETypesRegistry,
    inputs: &[Mapper::Field],
    locals: &[Mapper::Local],
    in_values: &[EValue],
    out_values: &mut Vec<EValue>,
) -> miette::Result<()> {
    out_values.clear();

    // Fill the outputs with the input values in the order of the IDs
    for (i, id) in locals.iter().enumerate() {
        let input_pos = if inputs.get(i).is_some_and(|f| mapper.matches(f, id)) {
            i
        } else if let Some(idx) = locals.iter().position(|f| f == id) {
            idx
        } else {
            bail!("Input {} was deleted", id);
        };

        out_values.push(
            in_values
                .get(input_pos)
                .cloned()
                .unwrap_or_else(|| mapper.default_value(&inputs[input_pos], registry)),
        );
    }

    Ok(())
}

/// Maps the outgoing values from the local fields representation to the
/// desired fields structure, filling the missing outputs with default values
pub fn map_outputs<Mapper: FieldMapper>(
    mapper: &Mapper,
    registry: &ETypesRegistry,
    outputs: &[Mapper::Field],
    ids: &[Mapper::Local],
    in_values: &[EValue],
    out_values: &mut Vec<EValue>,
) -> miette::Result<()> {
    out_values.clear();

    // Fill the group outputs with the incoming values, matching the order of the IDs
    // New outputs will be filled with default values
    for (i, field) in outputs.iter().enumerate() {
        let Some(input_pos) = (if ids.get(i).is_some_and(|id| mapper.matches(field, id)) {
            Some(i)
        } else {
            ids.iter().position(|f| mapper.matches(field, f))
        }) else {
            let default = mapper.default_value(field, registry);
            out_values.push(default);
            continue;
        };

        out_values.push(in_values[input_pos].clone());
    }

    // Check is any output was removed and incoming value now has no matching output
    for (i, id) in ids.iter().enumerate() {
        if outputs.get(i).is_some_and(|f| mapper.matches(f, id)) {
            continue;
        }
        if outputs.iter().any(|f| mapper.matches(f, id)) {
            continue;
        }

        bail!("Output {} was deleted", id);
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIs)]
pub enum IoDirection {
    Input,
    Output,
}

#[cfg(test)]
mod tests {
    use crate::etype::EDataType;
    use crate::graph::node::commands::SnarlCommands;
    use crate::graph::node::ports::fields::{
        get_field, sync_fields_and_types, FieldMapper, IoDirection,
    };
    use crate::registry::ETypesRegistry;
    use crate::value::EValue;
    use egui_snarl::NodeId;
    use itertools::Itertools;
    use rand::rngs::SmallRng;
    use rand::seq::SliceRandom;
    use rand::SeedableRng;

    struct Mapper;

    impl FieldMapper for Mapper {
        type Field = EDataType;
        type Local = String;
        type Type = EDataType;

        fn matches(&self, field: &Self::Field, local: &Self::Local) -> bool {
            field.name() == *local
        }

        fn to_local(&self, field: &Self::Field) -> Self::Local {
            field.name().to_string()
        }

        fn field_type(&self, field: &Self::Field) -> EDataType {
            *field
        }

        fn default_value(&self, field: &Self::Field, registry: &ETypesRegistry) -> EValue {
            self.field_type(field).default_value(registry).into_owned()
        }
    }

    fn reorder<T>(items: &mut [T], order: &mut [usize]) {
        for i in 0..order.len() {
            // We iterate through the arrows in the cycle. Keep track of the
            // element before and after the arrow. (left is before, right after)
            let mut left = i;
            let mut right = order[i];

            // Until we are back to the beginning, we swap.
            while right != i {
                // Swap the two elements.
                items.swap(left, right);
                // Mark the previous element as a length-one loop.
                order[left] = left;
                // Go to the next arrow.
                left = right;
                right = order[right];
            }
            // Mark the last element as a length-one loop as well.
            order[left] = left;
        }
    }

    fn perform_checks(fields: Vec<EDataType>, shuffle: impl FnOnce(&mut Vec<usize>)) {
        let mut indices = (0..fields.len()).collect_vec();
        shuffle(&mut indices);
        let locals_og = fields.iter().map(|f| Mapper.to_local(f)).collect_vec();
        let types_og = fields.iter().map(|f| Mapper.field_type(f)).collect_vec();
        let mut locals = locals_og.clone();
        let mut types = types_og.clone();
        reorder(&mut locals, &mut indices.clone());
        reorder(&mut types, &mut indices.clone());

        for (i, local) in locals.iter().enumerate() {
            let field = get_field(&Mapper, &fields, &locals, i).expect("Field should be found");
            assert_eq!(&Mapper.to_local(field), local);
        }

        let mut commands = SnarlCommands::default();
        sync_fields_and_types(
            &Mapper,
            &mut commands,
            &fields,
            &mut locals,
            Some(&mut types),
            NodeId(0),
            IoDirection::Input,
        );

        assert_eq!(locals, locals_og);
        assert_eq!(types, types_og);
    }

    #[test]
    fn check_unshuffled() {
        let fields = vec![EDataType::Boolean, EDataType::Number, EDataType::String];

        perform_checks(fields.clone(), |_| {});
    }

    #[test]
    fn check_reversed() {
        let fields = vec![EDataType::Boolean, EDataType::Number, EDataType::String];

        perform_checks(fields.clone(), |indices| indices.reverse());
    }

    #[test]
    fn check_shuffled() {
        let fields = vec![EDataType::Boolean, EDataType::Number, EDataType::String];

        let seeds = (0..usize::MAX).step_by(usize::MAX / 100000);
        for seed in seeds {
            println!("Seed: {}", seed);
            let mut rng = SmallRng::seed_from_u64(seed as u64);
            perform_checks(fields.clone(), |indices| indices.shuffle(&mut rng));
        }
    }
}
