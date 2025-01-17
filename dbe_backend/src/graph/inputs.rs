use crate::etype::EDataType;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct GraphInput {
    pub ty: Option<EDataType>,
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct GraphOutput {
    pub ty: Option<EDataType>,
    pub id: Uuid,
    pub name: String,
}

pub trait GraphIoData {
    fn id(&self) -> &Uuid;
    fn name(&self) -> &str;
    fn name_mut(&mut self) -> &mut String;
    fn ty(&self) -> Option<EDataType>;
}

impl GraphIoData for GraphInput {
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

impl GraphIoData for GraphOutput {
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
