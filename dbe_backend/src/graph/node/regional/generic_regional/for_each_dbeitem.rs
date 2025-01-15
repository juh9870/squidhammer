use crate::etype::EDataType;
use crate::graph::node::generic::macros::generic_node_io;
use crate::graph::node::regional::generic_regional::GenericRegionalNode;
use crate::graph::node::regional::{remember_variables, RegionIoKind};
use crate::graph::node::variables::ExecutionExtras;
use crate::graph::node::{ExecutionResult, NodeContext};
use crate::graph::region::{get_region_execution_data, RegionExecutionData};
use crate::project::ProjectFile;
use crate::value::EValue;
use itertools::Itertools;
use std::iter::Peekable;
use ustr::Ustr;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ForEachDbeItem {
    output_ty: Option<EDataType>,
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

    generic_node_io! {
        inputs {
            Start => [],
            End => []
        }
    }

    generic_node_io! {
        outputs {
            Start => [2;Value(self.output_ty), Fixed(EDataType::String)],
            End => [2;]
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

            let state = variables.get_or_init_region_data(region, |effects| {
                let files = effects
                    .project_files_iter()
                    .expect("side effects were checked for")
                    .filter_map(|(path, file)| {
                        let (ProjectFile::Value(value) | ProjectFile::GeneratedValue(value)) = file
                        else {
                            return None;
                        };

                        if value.ty() == ty {
                            Some((path.to_string(), value.clone()))
                        } else {
                            None
                        }
                    })
                    .collect_vec();

                ForEachDbeItemNodeState {
                    iter: files.into_iter().peekable(),
                    values: None,
                }
            });

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
        &["objects"]
    }

    fn create() -> Self {
        Self { output_ty: None }
    }
}

#[derive(Debug)]
struct ForEachDbeItemNodeState {
    iter: Peekable<std::vec::IntoIter<(String, EValue)>>,
    values: Option<Vec<EValue>>,
}

impl RegionExecutionData for ForEachDbeItemNodeState {}
