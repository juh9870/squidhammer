use crate::etype::eitem::EItemInfo;
use crate::etype::EDataType;
use crate::graph::node::commands::SnarlCommands;
use crate::graph::node::ports::{InputData, NodePortType, OutputData};
use crate::graph::node::regional::{RegionIoKind, RegionalNode};
use crate::graph::node::variables::ExecutionExtras;
use crate::graph::node::{ExecutionResult, NodeContext};
use crate::graph::region::RegionExecutionData;
use crate::json_utils::JsonValue;
use crate::registry::ETypesRegistry;
use crate::value::{ENumber, EValue};
use egui_snarl::{InPin, OutPin};
use miette::{bail, miette, IntoDiagnostic};
use std::ops::ControlFlow;
use ustr::Ustr;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ForEachRegionalNode {
    input_ty: Option<EDataType>,
    output_ty: Option<EDataType>,
}

impl RegionalNode for ForEachRegionalNode {
    fn id() -> Ustr {
        "for_each".into()
    }

    fn write_json(
        &self,
        _registry: &ETypesRegistry,
        kind: RegionIoKind,
    ) -> miette::Result<JsonValue> {
        if kind.is_start() {
            serde_json::to_value(self.input_ty).into_diagnostic()
        } else {
            serde_json::to_value(self.output_ty).into_diagnostic()
        }
    }

    fn parse_json(
        &mut self,
        _registry: &ETypesRegistry,
        kind: RegionIoKind,
        value: &mut JsonValue,
    ) -> miette::Result<()> {
        let ty = serde_json::from_value(value.take()).into_diagnostic()?;
        if kind.is_start() {
            self.input_ty = ty;
        } else {
            self.output_ty = ty;
        }

        Ok(())
    }

    fn inputs_count(&self, _context: NodeContext, _kind: RegionIoKind) -> usize {
        1
    }

    fn outputs_count(&self, _context: NodeContext, kind: RegionIoKind) -> usize {
        if kind.is_start() {
            2
        } else {
            1
        }
    }

    fn input_unchecked(
        &self,
        context: NodeContext,
        kind: RegionIoKind,
        _input: usize,
    ) -> miette::Result<InputData> {
        if kind.is_start() {
            Ok(InputData::new(
                if let Some(ty) = self.input_ty {
                    EItemInfo::simple_type(context.registry.list_of(ty)).into()
                } else {
                    NodePortType::BasedOnSource
                },
                "values".into(),
            ))
        } else {
            Ok(InputData::new(
                if let Some(ty) = self.output_ty {
                    EItemInfo::simple_type(ty).into()
                } else {
                    NodePortType::BasedOnSource
                },
                "output".into(),
            ))
        }
    }

    fn output_unchecked(
        &self,
        context: NodeContext,
        kind: RegionIoKind,
        output: usize,
    ) -> miette::Result<OutputData> {
        if kind.is_start() {
            match output {
                0 => Ok(OutputData::new(
                    if let Some(ty) = self.input_ty {
                        EItemInfo::simple_type(ty).into()
                    } else {
                        NodePortType::BasedOnTarget
                    },
                    "value".into(),
                )),
                1 => Ok(OutputData::new(
                    EItemInfo::simple_type(EDataType::Number).into(),
                    "index".into(),
                )),
                _ => {
                    bail!("Invalid output index: {}", output);
                }
            }
        } else {
            Ok(OutputData::new(
                if let Some(ty) = self.output_ty {
                    EItemInfo::simple_type(context.registry.list_of(ty)).into()
                } else {
                    NodePortType::BasedOnTarget
                },
                "output".into(),
            ))
        }
    }

    fn try_connect(
        &mut self,
        context: NodeContext,
        kind: RegionIoKind,
        _commands: &mut SnarlCommands,
        _from: &OutPin,
        to: &InPin,
        incoming_type: &NodePortType,
    ) -> miette::Result<ControlFlow<bool>> {
        if to.id.input != 0 {
            bail!("Invalid input index: {}", to.id.input);
        }
        if kind.is_start() {
            if self.input_ty.is_some() {
                return Ok(ControlFlow::Continue(()));
            }

            let EDataType::List { id } = incoming_type.ty() else {
                return Ok(ControlFlow::Break(false));
            };

            let list = context
                .registry
                .get_list(&id)
                .ok_or_else(|| miette!("Unknown list type: {}", id))?;

            let ty = list.value_type;

            self.input_ty = Some(ty);

            Ok(ControlFlow::Continue(()))
        } else {
            if self.output_ty.is_some() {
                return Ok(ControlFlow::Continue(()));
            }

            if !incoming_type.is_specific() {
                return Ok(ControlFlow::Break(false));
            }

            self.output_ty = Some(incoming_type.ty());

            Ok(ControlFlow::Continue(()))
        }
    }

    fn can_output_to(
        &self,
        _context: NodeContext,
        kind: RegionIoKind,
        _from: &OutPin,
        to: &InPin,
        target_type: &NodePortType,
    ) -> miette::Result<bool> {
        if to.id.input != 0 {
            bail!("Invalid input index: {}", to.id.input);
        }
        if kind.is_start() {
            if self.input_ty.is_some() {
                bail!("Input type already set");
            }
        } else {
            if self.output_ty.is_some() {
                bail!("Output type already set");
            }

            if !target_type.ty().is_list() {
                return Ok(false);
            };
        }
        Ok(true)
    }

    fn connected_to_output(
        &mut self,
        context: NodeContext,
        kind: RegionIoKind,
        _commands: &mut SnarlCommands,
        _from: &OutPin,
        to: &InPin,
        incoming_type: &NodePortType,
    ) -> miette::Result<()> {
        if to.id.input != 0 {
            bail!("Invalid input index: {}", to.id.input);
        }

        if kind.is_start() {
            if self.input_ty.is_some() {
                bail!("Input type already set");
            };

            self.input_ty = Some(incoming_type.ty());

            Ok(())
        } else {
            if self.output_ty.is_some() {
                bail!("Output type already set");
            };

            let EDataType::List { id } = incoming_type.ty() else {
                bail!("Expected list type, got: {}", incoming_type.ty().name());
            };

            let list = context
                .registry
                .get_list(&id)
                .ok_or_else(|| miette!("Unknown list type: {}", id))?;

            let ty = list.value_type;

            self.output_ty = Some(ty);

            Ok(())
        }
    }

    fn should_execute(
        &self,
        _context: NodeContext,
        region: Uuid,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<bool> {
        let Some(state) = variables.get_region_data::<ForEachNodeState>(region) else {
            bail!("End of for-each node without start")
        };

        Ok(state.index < state.length)
    }

    fn execute(
        &self,
        context: NodeContext,
        kind: RegionIoKind,
        region: Uuid,
        inputs: &[EValue],
        outputs: &mut Vec<EValue>,
        variables: &mut ExecutionExtras,
    ) -> miette::Result<ExecutionResult> {
        if kind.is_start() {
            let EValue::List { values, .. } = &inputs[0] else {
                bail!("Expected list input, got: {}", inputs[0].ty().name());
            };
            let state = variables.get_or_init_region_data(region, || ForEachNodeState {
                index: 0,
                length: values.len(),
                output: Vec::with_capacity(values.len()),
                values: None,
            });

            outputs.clear();
            outputs.push(values[state.index].clone());
            outputs.push(ENumber::from(state.index as f64).into());

            if let Some(values) = state.values.take() {
                outputs.extend(values);
            } else {
                outputs.extend(inputs.iter().skip(1).cloned());
            }

            Ok(ExecutionResult::Done)
        } else {
            let Some(state) = variables.get_region_data::<ForEachNodeState>(region) else {
                bail!("End of for_each node without start")
            };

            state.index += 1;
            state.output.push(inputs[0].clone());

            if state.index >= state.length {
                outputs.clear();
                outputs.push(EValue::List {
                    values: std::mem::take(&mut state.output),
                    id: context
                        .registry
                        .list_id_of(self.output_ty.unwrap_or_else(EDataType::null)),
                });
                outputs.extend(inputs.iter().skip(1).cloned());
                variables.remove_region_data(region);
                Ok(ExecutionResult::Done)
            } else {
                state.values = Some(inputs.to_vec());
                Ok(ExecutionResult::RerunRegion { region })
            }
        }
    }

    fn categories() -> &'static [&'static str] {
        &["utility"]
    }

    fn create() -> Self {
        Self {
            input_ty: None,
            output_ty: None,
        }
    }
}

#[derive(Debug)]
struct ForEachNodeState {
    index: usize,
    length: usize,
    output: Vec<EValue>,
    values: Option<Vec<EValue>>,
}

impl RegionExecutionData for ForEachNodeState {}
