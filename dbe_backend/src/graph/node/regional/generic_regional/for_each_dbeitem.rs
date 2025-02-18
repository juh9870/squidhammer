use crate::etype::eenum::variant::EEnumVariantId;
use crate::etype::EDataType;
use crate::graph::node::commands::{SnarlCommand, SnarlCommands};
use crate::graph::node::editable_state::{EditableState, EditableStateValue};
use crate::graph::node::extras::ExecutionExtras;
use crate::graph::node::generic::macros::generic_node_io;
use crate::graph::node::generic::{GenericNodeField, GenericNodeFieldMut};
use crate::graph::node::regional::{NodeWithVariables, RegionIoData, RegionIoKind};
use crate::graph::node::stateful::generic::GenericStatefulNode;
use crate::graph::node::variables::remember_variables;
use crate::graph::node::{ExecutionResult, NodeContext};
use crate::graph::region::{get_region_execution_data, RegionExecutionData};
use crate::json_utils::JsonValue;
use crate::project::ProjectFile;
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use egui_snarl::{NodeId, OutPinId};
use itertools::Itertools;
use miette::{bail, miette};
use serde::{Deserialize, Serialize};
use smallvec::smallvec;
use std::iter::Peekable;
use std::ops::Deref;
use ustr::Ustr;
use utils::smallvec_n;

#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
pub struct ForEachDbeItem {
    output_ty: Option<EDataType>,
    is_enum: bool,
    enum_variant: Option<(EDataType, EEnumVariantId)>,
}

impl NodeWithVariables for ForEachDbeItem {
    type State<'a> = &'a RegionIoData;
}

impl GenericStatefulNode for ForEachDbeItem {
    type State<'a> = &'a RegionIoData;

    fn id() -> Ustr {
        "for_each_dbeitem".into()
    }

    fn input_names(&self, _data: &&RegionIoData) -> &[&str] {
        &[]
    }
    fn output_names(&self, data: &&RegionIoData) -> &[&str] {
        match data.kind {
            RegionIoKind::Start => &["value", "path"],
            RegionIoKind::End => &[],
        }
    }

    fn outputs(
        &self,
        _registry: &ETypesRegistry,
        external_state: &Self::State<'_>,
    ) -> impl AsRef<[GenericNodeField]> {
        match external_state.kind {
            RegionIoKind::Start => {
                if let Some((ty, _)) = self.enum_variant {
                    smallvec_n![2;
                        GenericNodeField::Fixed(ty),
                        GenericNodeField::Fixed(EDataType::String),
                    ]
                } else {
                    smallvec_n![2;
                        GenericNodeField::Value(&self.output_ty),
                        GenericNodeField::Fixed(EDataType::String),
                    ]
                }
            }
            RegionIoKind::End => {
                smallvec![]
            }
        }
    }

    fn outputs_mut(
        &mut self,
        _registry: &ETypesRegistry,
        external_state: &Self::State<'_>,
    ) -> impl AsMut<[GenericNodeFieldMut]> {
        match external_state.kind {
            RegionIoKind::Start => {
                if let Some((ty, _)) = self.enum_variant {
                    smallvec_n![2;
                        GenericNodeFieldMut::Fixed(ty),
                        GenericNodeFieldMut::Fixed(EDataType::String),
                    ]
                } else {
                    smallvec_n![2;
                        GenericNodeFieldMut::Value(&mut self.output_ty),
                        GenericNodeFieldMut::Fixed(EDataType::String),
                    ]
                }
            }
            RegionIoKind::End => smallvec![],
        }
    }

    generic_node_io! {
        inputs {
            Start => [],
            End => []
        }
    }

    fn write_json(
        &self,
        _registry: &crate::registry::ETypesRegistry,
        data: &RegionIoData,
    ) -> miette::Result<JsonValue> {
        if data.is_start() {
            miette::IntoDiagnostic::into_diagnostic(serde_json::value::to_value(self))
        } else {
            Ok(JsonValue::Null)
        }
    }
    fn parse_json(
        &mut self,
        _registry: &crate::registry::ETypesRegistry,
        data: &RegionIoData,
        value: &mut JsonValue,
    ) -> miette::Result<()> {
        if data.is_end() {
            return Ok(());
        }

        miette::IntoDiagnostic::into_diagnostic(Self::deserialize(value.take()))
            .map(|node| *self = node)
    }

    fn has_editable_state(&self, data: &RegionIoData) -> bool {
        data.is_start() && self.output_ty.is_some() && self.is_enum
    }

    fn editable_state(&self, data: &RegionIoData) -> EditableState {
        if !data.is_start() {
            unimplemented!()
        }

        let mut vec = smallvec![(
            "filter_by_variant".into(),
            EditableStateValue::Value(self.enum_variant.is_some().into())
        ),];
        if let Some((_, variant)) = self.enum_variant {
            vec.push((
                "enum_variant".into(),
                EditableStateValue::EnumVariant(variant),
            ));
        }

        vec
    }

    fn apply_editable_state(
        &mut self,
        context: NodeContext,
        data: &RegionIoData,
        state: EditableState,
        commands: &mut SnarlCommands,
        node_id: NodeId,
    ) -> miette::Result<()> {
        if !data.is_start() {
            unimplemented!()
        }

        let mut state = state.into_iter();

        let (_, filter_by_variant) = state.next().unwrap();
        let filter_by_variant = filter_by_variant.try_as_value().unwrap();
        let filter_by_variant = filter_by_variant.try_as_boolean()?;

        if !filter_by_variant {
            if self.enum_variant.is_some() {
                commands.push(SnarlCommand::ReconnectOutput {
                    id: OutPinId {
                        node: node_id,
                        output: 0,
                    },
                });
            }
            self.enum_variant = None;
            return Ok(());
        }

        let EDataType::Object {
            ident: object_ident,
        } = self.output_ty.unwrap()
        else {
            bail!("Is not an enum")
        };

        let variant_id = if let Some((_, variant_id)) = state.next() {
            variant_id.try_as_enum_variant().unwrap()
        } else {
            let Some(data) = context.registry.get_enum(&object_ident) else {
                bail!("Object is not an enum")
            };

            *data
                .variant_ids()
                .iter()
                .next()
                .ok_or_else(|| miette!("Enum has no variants"))?
        };

        let Some((data, variant)) = variant_id.enum_variant(context.registry) else {
            bail!("Invalid enum variant")
        };

        if object_ident != data.ident {
            bail!(
                "Enum variant {} does not match output type `{}`",
                variant_id,
                object_ident
            )
        }

        if self.enum_variant.is_none() {
            commands.push(SnarlCommand::ReconnectOutput {
                id: OutPinId {
                    node: node_id,
                    output: 0,
                },
            });
        }

        self.enum_variant = Some((variant.data.ty(), variant_id));

        Ok(())
    }

    fn types_changed(
        &mut self,
        context: NodeContext,
        external_state: &RegionIoData,
        _node: NodeId,
        _commands: &mut SnarlCommands,
    ) {
        if !external_state.is_start() {
            return;
        }
        self.is_enum = match self.output_ty {
            Some(EDataType::Object { ident }) => context.registry.get_enum(&ident).is_some(),
            _ => false,
        };
        if !self.is_enum {
            self.enum_variant = None;
        }
    }

    fn should_execute(
        &self,
        _context: NodeContext,
        region: &RegionIoData,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<bool> {
        let state = get_region_execution_data::<ForEachDbeItemNodeState>(region.region, variables)?;

        Ok(state.had_value)
    }

    fn execute(
        &self,
        _context: NodeContext,
        region: &RegionIoData,
        inputs: &[EValue],
        outputs: &mut Vec<EValue>,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<ExecutionResult> {
        if region.is_start() {
            let Some(ty) = self.output_ty else {
                variables.get_or_init_region_data(region.region, |_| ForEachDbeItemNodeState {
                    iter: vec![].into_iter().peekable(),
                    had_value: false,
                    values: None,
                });
                return Ok(ExecutionResult::Done);
            };

            if !variables.side_effects.is_available() {
                variables.get_or_init_region_data(region.region, |_| ForEachDbeItemNodeState {
                    iter: vec![].into_iter().peekable(),
                    had_value: false,
                    values: None,
                });
                return Ok(ExecutionResult::Done);
            }

            let state = variables.get_or_try_init_region_data(region.region, |effects| {
                let files: Vec<_> = effects
                    .project_files_iter()
                    .expect("side effects were checked for")
                    .filter_map(|(path, file)| {
                        let (ProjectFile::Value(value) | ProjectFile::GeneratedValue(value)) = file
                        else {
                            return None;
                        };

                        if value.ty() == ty {
                            let Some((ty, expected_variant)) = self.enum_variant else {
                                return Some(Ok((path.to_string(), value.clone())));
                            };

                            let EValue::Enum { variant, data } = value else {
                                return Some(Err(miette!("Expected enum value")));
                            };

                            if variant == &expected_variant {
                                debug_assert_eq!(ty, data.ty());

                                Some(Ok((path.to_string(), data.deref().clone())))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .try_collect()?;

                Ok(ForEachDbeItemNodeState {
                    iter: files.into_iter().peekable(),
                    had_value: false,
                    values: None,
                })
            })?;

            state.had_value = false;

            outputs.clear();
            if let Some((path, value)) = state.iter.next() {
                outputs.push(value);
                outputs.push(path.into());
                state.had_value = true;
            }

            remember_variables(&mut state.values, inputs, outputs);

            Ok(ExecutionResult::Done)
        } else {
            let state =
                get_region_execution_data::<ForEachDbeItemNodeState>(region.region, variables)?;

            if state.iter.peek().is_none() {
                outputs.extend(inputs.iter().cloned());
                variables.remove_region_data(region.region);
                Ok(ExecutionResult::Done)
            } else {
                state.values = Some(inputs[..].to_vec());
                Ok(ExecutionResult::RerunRegion {
                    region: region.region,
                })
            }
        }
    }

    fn categories() -> &'static [&'static str] {
        &["objects", "utility.iterators"]
    }

    fn create() -> Self {
        Self {
            output_ty: None,
            is_enum: false,
            enum_variant: None,
        }
    }
}

#[derive(Debug)]
struct ForEachDbeItemNodeState {
    iter: Peekable<std::vec::IntoIter<(String, EValue)>>,
    had_value: bool,
    values: Option<Vec<EValue>>,
}

impl RegionExecutionData for ForEachDbeItemNodeState {}
