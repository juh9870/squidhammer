use crate::etype::eenum::variant::EEnumVariantId;
use crate::etype::EDataType;
use crate::graph::node::commands::{SnarlCommand, SnarlCommands};
use crate::graph::node::editable_state::{EditableState, EditableStateValue};
use crate::graph::node::generic::macros::generic_node_io;
use crate::graph::node::generic::{GenericNodeField, GenericNodeFieldMut};
use crate::graph::node::regional::generic_regional::GenericRegionalNode;
use crate::graph::node::regional::{remember_variables, RegionIoKind};
use crate::graph::node::variables::ExecutionExtras;
use crate::graph::node::{ExecutionResult, NodeContext};
use crate::graph::region::{get_region_execution_data, RegionExecutionData};
use crate::json_utils::JsonValue;
use crate::project::ProjectFile;
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
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForEachDbeItem {
    output_ty: Option<EDataType>,
    is_enum: bool,
    enum_variant: Option<(EDataType, EEnumVariantId)>,
}

impl GenericRegionalNode for ForEachDbeItem {
    fn id() -> Ustr {
        "for_each_dbeitem".into()
    }

    fn input_names(&self, kind: RegionIoKind) -> &[&str] {
        match kind {
            RegionIoKind::Start => &[],
            RegionIoKind::End => &[],
        }
    }
    fn output_names(&self, kind: RegionIoKind) -> &[&str] {
        match kind {
            RegionIoKind::Start => &["value", "path"],
            RegionIoKind::End => &[],
        }
    }

    fn outputs(&self, kind: RegionIoKind) -> impl AsRef<[GenericNodeField]> {
        match kind {
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

    fn outputs_mut(&mut self, kind: RegionIoKind) -> impl AsMut<[GenericNodeFieldMut]> {
        match kind {
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
        kind: RegionIoKind,
    ) -> miette::Result<JsonValue> {
        if kind.is_start() {
            miette::IntoDiagnostic::into_diagnostic(serde_json::value::to_value(self))
        } else {
            Ok(JsonValue::Null)
        }
    }
    fn parse_json(
        &mut self,
        _registry: &crate::registry::ETypesRegistry,
        kind: RegionIoKind,
        value: &mut JsonValue,
    ) -> miette::Result<()> {
        if kind.is_end() {
            return Ok(());
        }

        miette::IntoDiagnostic::into_diagnostic(Self::deserialize(value.take()))
            .map(|node| *self = node)
    }

    fn has_editable_state(&self, kind: RegionIoKind) -> bool {
        kind.is_start() && self.output_ty.is_some() && self.is_enum
    }

    fn editable_state(&self, kind: RegionIoKind) -> EditableState {
        if !kind.is_start() {
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
        kind: RegionIoKind,
        state: EditableState,
        commands: &mut SnarlCommands,
        node_id: NodeId,
    ) -> miette::Result<()> {
        if !kind.is_start() {
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
                })
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
            })
        }

        self.enum_variant = Some((variant.data.ty(), variant_id));

        Ok(())
    }

    fn types_changed(
        &mut self,
        context: NodeContext,
        kind: RegionIoKind,
        _region: Uuid,
        _node: NodeId,
        _commands: &mut SnarlCommands,
    ) {
        if !kind.is_start() {
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
        region: Uuid,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<bool> {
        let state = get_region_execution_data::<ForEachDbeItemNodeState>(region, variables)?;

        Ok(state.iter.peek().is_some())
    }

    fn execute(
        &self,
        _context: NodeContext,
        kind: RegionIoKind,
        region: Uuid,
        inputs: &[EValue],
        outputs: &mut Vec<EValue>,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<ExecutionResult> {
        if kind.is_start() {
            let Some(ty) = self.output_ty else {
                variables.get_or_init_region_data(region, |_| ForEachDbeItemNodeState {
                    iter: vec![].into_iter().peekable(),
                    values: None,
                });
                return Ok(ExecutionResult::Done);
            };

            if !variables.side_effects.is_available() {
                variables.get_or_init_region_data(region, |_| ForEachDbeItemNodeState {
                    iter: vec![].into_iter().peekable(),
                    values: None,
                });
                return Ok(ExecutionResult::Done);
            }

            let state = variables.get_or_try_init_region_data(region, |effects| {
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
                    values: None,
                })
            })?;

            outputs.clear();
            if let Some((path, value)) = state.iter.next() {
                outputs.push(value);
                outputs.push(path.into());
            }

            remember_variables(&mut state.values, &inputs[1..], outputs);

            Ok(ExecutionResult::Done)
        } else {
            let state = get_region_execution_data::<ForEachDbeItemNodeState>(region, variables)?;

            if state.iter.peek().is_none() {
                outputs.extend(inputs.iter().cloned());
                variables.remove_region_data(region);
                Ok(ExecutionResult::Done)
            } else {
                state.values = Some(inputs[..].to_vec());
                Ok(ExecutionResult::RerunRegion { region })
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
    values: Option<Vec<EValue>>,
}

impl RegionExecutionData for ForEachDbeItemNodeState {}
