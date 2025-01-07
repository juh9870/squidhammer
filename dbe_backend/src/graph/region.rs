use crate::etype::EDataType;
use crate::graph::inputs::GraphIoData;
use downcast_rs::{impl_downcast, Downcast};
use smallvec::SmallVec;
use uuid::Uuid;
pub mod region_graph;

#[derive(Debug)]
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

    fn ty(&self) -> Option<EDataType> {
        self.ty
    }
}

#[derive(Debug)]
pub struct RegionInfo {
    id: Uuid,
    pub variables: SmallVec<[RegionVariable; 1]>,
}

impl RegionInfo {
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            variables: Default::default(),
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
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
