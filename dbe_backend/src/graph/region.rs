use crate::etype::EDataType;
use crate::graph::inputs::GraphIoData;
use crate::graph::node::extras::ExecutionExtras;
use downcast_rs::{impl_downcast, Downcast};
use miette::bail;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use utils::color_format::ecolor::Color32;
use uuid::Uuid;

pub mod region_graph;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionVariable {
    pub ty: Option<EDataType>,
    pub id: Uuid,
    pub name: String,
}

impl GraphIoData for RegionVariable {
    fn id(&self) -> &Uuid {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }

    fn ty(&self) -> Option<EDataType> {
        self.ty
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionInfo {
    id: Uuid,
    pub color: Option<Color32>,
    pub variables: SmallVec<[RegionVariable; 1]>,
}

impl RegionInfo {
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            color: None,
            variables: Default::default(),
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn color(&self) -> Color32 {
        self.color.unwrap_or_else(|| {
            let c = random_color::RandomColor::new()
                .seed(self.id.to_string())
                .luminosity(random_color::options::Luminosity::Light)
                .to_rgb_array();

            Color32::from_rgb(c[0], c[1], c[2])
        })
    }
}

// pub fn can_connect_regions(
//     regions: &AHashMap<Uuid, RegionInfo>,
//     from: Option<Uuid>,
//     to: Option<Uuid>,
// ) -> bool {
//     let (Some(from), Some(to)) = (from, to) else {
//         return true;
//     };
//
//     is_region_same_or_child_of(regions, to, from)
// }

// pub fn is_region_same_or_child_of(
//     regions: &AHashMap<Uuid, RegionInfo>,
//     region: Uuid,
//     parent: Uuid,
// ) -> bool {
//     if region == parent {
//         return true;
//     }
//     let Some(child) = regions.get(&region) else {
//         return false;
//     };
//
//     if child.parent_region == Some(parent) {
//         return true;
//     }
//
//     if let Some(parent) = child.parent_region {
//         return is_region_same_or_child_of(regions, parent, parent);
//     }
//
//     false
// }

pub trait RegionExecutionData: Downcast {}

impl_downcast!(RegionExecutionData);

pub fn get_region_execution_data<'a, T: RegionExecutionData>(
    region: Uuid,
    variables: &'a mut ExecutionExtras,
) -> miette::Result<&'a mut T> {
    let Some(state) = variables.get_region_data::<T>(region) else {
        bail!("End of regional node without start")
    };
    Ok(state)
}
