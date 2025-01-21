use crate::etype::eitem::EItemInfo;
use crate::etype::EDataType;
use crate::graph::inputs::{GraphInput, GraphIoData, GraphOutput};
use crate::graph::node::commands::SnarlCommands;
use crate::graph::node::ports::fields::{
    get_field, get_field_index, map_inputs, sync_fields_and_types, FieldMapper, IoDirection,
};
use crate::graph::node::ports::{InputData, NodePortType, OutputData};
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use egui_snarl::NodeId;
use std::marker::PhantomData;
use uuid::Uuid;

pub struct GraphIoMapper<IO: GraphIoData>(PhantomData<fn() -> IO>);

impl<IO: GraphIoData> GraphIoMapper<IO> {
    pub const INSTANCE: Self = Self(PhantomData);
}

impl<IO: GraphIoData> FieldMapper for GraphIoMapper<IO> {
    type Field = IO;
    type Local = Uuid;
    type Type = EDataType;

    fn matches(&self, field: &Self::Field, local: &Self::Local) -> bool {
        field.id() == local
    }

    fn to_local(&self, field: &Self::Field) -> Self::Local {
        *field.id()
    }

    fn field_type(&self, field: &Self::Field) -> EDataType {
        field.ty().unwrap_or_else(EDataType::null)
    }

    fn default_value(&self, field: &Self::Field, registry: &ETypesRegistry) -> EValue {
        self.field_type(field).default_value(registry).into_owned()
    }
}

/// Gets the field at the given index, using the ID lookup table to account for
/// potential reordering
///
/// # Arguments
/// * `fields` - The fields to get the data from
/// * `ids` - local index-to-ID lookup table
/// * `index` - The index of the field to get
pub fn get_graph_io_field<'ctx, IO: GraphIoData>(
    fields: &'ctx [IO],
    ids: &[Uuid],
    index: usize,
) -> Option<&'ctx IO> {
    get_field(&GraphIoMapper::<IO>::INSTANCE, fields, ids, index)
}

pub fn get_graph_io_field_index<IO: GraphIoData>(
    fields: &[IO],
    ids: &[Uuid],
    index: usize,
) -> Option<usize> {
    get_field_index(&GraphIoMapper::<IO>::INSTANCE, fields, ids, index)
}

/// Synchronizes the inputs or outputs of a node and updates the ID lookup
/// table to match the new order and presence of fields
///
/// # Arguments
/// * `commands` - Command buffer
/// * `fields` - The new fields to synchronize with
/// * `ids` - local index-to-ID lookup table
/// * `types` - types table to update
/// * `node_id` - The ID of the node
pub fn sync_fields<IO: GraphIoData, Store: AsRef<[Uuid]> + FromIterator<Uuid>>(
    commands: &mut SnarlCommands,
    fields: &[IO],
    ids: &mut Store,
    types: Option<&mut Vec<EDataType>>,
    node_id: NodeId,
    direction: IoDirection,
) {
    sync_fields_and_types(
        &GraphIoMapper::<IO>::INSTANCE,
        commands,
        fields,
        ids,
        types,
        node_id,
        direction,
    )
}

pub fn map_group_inputs(
    registry: &ETypesRegistry,
    inputs: &[GraphInput],
    ids: &[Uuid],
    in_values: &[EValue],
    out_values: &mut Vec<EValue>,
) -> miette::Result<()> {
    map_inputs(
        &GraphIoMapper::<GraphInput>::INSTANCE,
        registry,
        inputs,
        ids,
        in_values,
        out_values,
    )
}

pub fn map_group_outputs(
    registry: &ETypesRegistry,
    outputs: &[GraphOutput],
    ids: &[Uuid],
    in_values: &[EValue],
    out_values: &mut Vec<EValue>,
) -> miette::Result<()> {
    map_inputs(
        &GraphIoMapper::<GraphOutput>::INSTANCE,
        registry,
        outputs,
        ids,
        in_values,
        out_values,
    )
}

pub fn get_port_input<IO: GraphIoData>(
    fields: &[IO],
    ids: &[Uuid],
    index: usize,
) -> miette::Result<InputData> {
    let Some(f) = get_graph_io_field(fields, ids, index) else {
        return Ok(InputData::new(
            NodePortType::Invalid,
            "!!unknown input!!".into(),
        ));
    };

    Ok(InputData::new(
        f.ty()
            .map(EItemInfo::simple_type)
            .map(NodePortType::Specific)
            .unwrap_or_else(|| NodePortType::BasedOnSource),
        f.name().into(),
    ))
}

pub fn get_port_output<IO: GraphIoData>(
    fields: &[IO],
    ids: &[Uuid],
    index: usize,
) -> miette::Result<OutputData> {
    let Some(f) = get_graph_io_field(fields, ids, index) else {
        return Ok(OutputData::new(
            NodePortType::Invalid,
            "!!unknown output!!".into(),
        ));
    };

    Ok(OutputData::new(
        f.ty()
            .map(EItemInfo::simple_type)
            .map(NodePortType::Specific)
            .unwrap_or_else(|| NodePortType::BasedOnTarget),
        f.name().into(),
    ))
}
