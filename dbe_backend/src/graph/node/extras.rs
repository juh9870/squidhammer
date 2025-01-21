use crate::graph::region::RegionExecutionData;
use crate::project::side_effects::SideEffectsContext;
use crate::value::EValue;
use miette::{bail, miette};
use std::collections::hash_map::Entry;
use utils::map::HashMap;
use uuid::Uuid;

#[derive(derive_more::Debug)]
pub struct ExecutionExtras<'a> {
    is_node_group: bool,
    group_inputs: &'a [EValue],
    group_outputs: &'a mut Option<Vec<EValue>>,
    #[debug("(...)")]
    regional_data: &'a mut HashMap<Uuid, Box<dyn RegionExecutionData>>,
    pub side_effects: SideEffectsContext<'a>,
}

impl<'a> ExecutionExtras<'a> {
    pub fn new(
        is_node_group: bool,
        group_inputs: &'a [EValue],
        group_outputs: &'a mut Option<Vec<EValue>>,
        regional_data: &'a mut HashMap<Uuid, Box<dyn RegionExecutionData>>,
        side_effects: SideEffectsContext<'a>,
    ) -> Self {
        Self {
            is_node_group,
            group_inputs,
            group_outputs,
            regional_data,
            side_effects,
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

    pub fn get_or_init_region_data<T: RegionExecutionData>(
        &mut self,
        region: Uuid,
        init: impl FnOnce(&mut SideEffectsContext) -> T,
    ) -> &mut T {
        self.regional_data
            .entry(region)
            .or_insert_with(|| Box::new(init(&mut self.side_effects)))
            .downcast_mut::<T>()
            .expect("Region data type mismatch")
    }

    pub fn get_or_try_init_region_data<T: RegionExecutionData>(
        &mut self,
        region: Uuid,
        init: impl FnOnce(&mut SideEffectsContext) -> miette::Result<T>,
    ) -> miette::Result<&mut T> {
        let e = match self.regional_data.entry(region) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(Box::new(init(&mut self.side_effects)?)),
        };

        Ok(e.downcast_mut::<T>().expect("Region data type mismatch"))
    }

    pub fn get_region_data<T: RegionExecutionData>(&mut self, region: Uuid) -> Option<&mut T> {
        self.regional_data
            .get_mut(&region)
            .map(|data| data.downcast_mut::<T>().expect("Region data type mismatch"))
    }

    pub fn remove_region_data(&mut self, region: Uuid) {
        self.regional_data.remove(&region);
    }
}
