use crate::value::EValue;
use crate::MyGraphState;
use egui_node_graph::DataTypeTrait;
use rust_i18n::t;
use std::borrow::Cow;

/// `DataType`s are what defines the possible range of connections when
/// attaching two ports together. The graph UI will make sure to not allow
/// attaching incompatible datatypes.
#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MyDataType {
    Scalar,
    Vec2,
}

impl MyDataType {
    pub fn default_value(&self) -> EValue {
        match self {
            MyDataType::Scalar => EValue::Scalar { value: 0.0 },
            MyDataType::Vec2 => EValue::Vec2 {
                value: Default::default(),
            },
        }
    }
}

// A trait for the data types, to tell the library how to display them
impl DataTypeTrait<MyGraphState> for MyDataType {
    fn data_type_color(&self, _user_state: &mut MyGraphState) -> egui::Color32 {
        match self {
            MyDataType::Scalar => egui::Color32::from_rgb(38, 109, 211),
            MyDataType::Vec2 => egui::Color32::from_rgb(238, 207, 109),
        }
    }

    fn name(&self) -> Cow<'_, str> {
        match self {
            MyDataType::Scalar => Cow::Owned(t!("scalar")),
            MyDataType::Vec2 => Cow::Owned(t!("vec")),
        }
    }
}
