use crate::value::EValue;
use miette::{bail, miette};

#[derive(Debug)]
pub struct ExecutionVariables<'a> {
    is_node_group: bool,
    group_inputs: &'a [EValue],
    group_outputs: &'a mut Option<Vec<EValue>>,
}

impl<'a> ExecutionVariables<'a> {
    pub fn new(
        is_node_group: bool,
        group_inputs: &'a [EValue],
        group_outputs: &'a mut Option<Vec<EValue>>,
    ) -> Self {
        Self {
            is_node_group,
            group_inputs,
            group_outputs,
        }
    }

    pub fn get_inputs(&self) -> miette::Result<&[EValue]> {
        if !self.is_node_group {
            bail!("Can't get input values from non-group graph");
        }
        Ok(self.group_inputs)
    }

    /// Gets the input value of the node at the index
    pub fn get_input(&self, index: usize) -> miette::Result<&EValue> {
        if !self.is_node_group {
            bail!("Can't get input value from non-group graph");
        }
        self.group_inputs.get(index).ok_or_else(|| {
            miette!(
                "Input index {} out of bounds, group only has {} inputs",
                index,
                self.group_inputs.len()
            )
        })
    }

    pub fn set_outputs(&mut self, values: Vec<EValue>) -> miette::Result<()> {
        if !self.is_node_group {
            bail!("Can't set output of non-group graph");
        }

        if self.group_outputs.is_some() {
            bail!("Outputs already set");
        }

        *self.group_outputs = Some(values);

        Ok(())
    }
}
