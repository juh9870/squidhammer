use crate::graph::nodes::data::EditorNodeData;
use crate::graph::EditorGraphResponse;
use crate::value::etype::registry::eenum::EEnumVariantId;
use crate::value::etype::registry::ETypetId;
use crate::EditorGraphState;
use egui_node_graph::{NodeId, WidgetValueTrait};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use smallvec::{Array, SmallVec};
use std::fmt::{Display, Formatter};
use ustr::UstrMap;

pub use serde_json::Value as JsonValue;

pub mod connections;
pub mod draw;
pub mod etype;

#[cfg(not(feature = "f64"))]
pub type ENumber = f32;
#[cfg(feature = "f64")]
pub type ENumber = f64;

#[cfg(not(feature = "f64"))]
pub type EVector2 = glam::f32::Vec2;

#[cfg(feature = "f64")]
pub type EVector2 = glam::f64::Vec2;

/// In the graph, input parameters can optionally have a constant value. This
/// value can be directly edited in a widget inside the node itself.
///
/// There will usually be a correspondence between DataTypes and ValueTypes. But
/// this library makes no attempt to check this consistency. For instance, it is
/// up to the user code in this example to make sure no parameter is created
/// with a DataType of Scalar and a ValueType of Vec2.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum EValue {
    Unknown {
        value: JsonValue,
    },
    Boolean {
        value: bool,
    },
    Scalar {
        value: ENumber,
    },
    Vec2 {
        value: EVector2,
    },
    String {
        value: String,
    },
    Struct {
        ident: ETypetId,
        fields: UstrMap<EValue>,
    },
    Id {
        ty: ETypetId,
        value: Option<ETypetId>,
    },
    Ref {
        ty: ETypetId,
        value: Option<ETypetId>,
    },
    Enum {
        variant: EEnumVariantId,
        data: Box<EValue>,
    },
}

impl Default for EValue {
    fn default() -> Self {
        // NOTE: This is just a dummy `Default` implementation. The library
        // requires it to circumvent some internal borrow checker issues.
        Self::Scalar { value: 0.0 }
    }
}

#[derive(Debug, Clone)]
pub struct EValueInputWrapper<'a>(pub SmallVec<[&'a EValue; 1]>);

macro_rules! try_to {
    ($type:tt, $result:ty, $name:ident) => {
        paste::item! {
            impl TryFrom<EValue> for $result {
                type Error = anyhow::Error;

                fn try_from(value: EValue) -> Result<Self, Self::Error> {
                    value.[<try_into_ $name>]()
                }
            }

            impl TryFrom<&EValue> for $result {
                type Error = anyhow::Error;

                fn try_from(value: &EValue) -> Result<Self, Self::Error> {
                    value.[<try_as_ $name>]().map(|e|e.clone())
                }
            }

            impl<'a> TryFrom<EValueInputWrapper<'a>> for $result {
                type Error = anyhow::Error;

                fn try_from(value: EValueInputWrapper<'a>) -> Result<Self, Self::Error> {
                    if value.0.len() != 1 {
                        anyhow::bail!("Got {} inputs where only one was expected.", value.0.len());
                    }

                    Self::try_from(value.0[0])
                }
            }

            impl From<$result> for EValue {
                fn from(value: $result) -> Self {
                    Self::$type{value}
                }
            }

            impl EValue {
                pub fn [<try_into_ $name>](self) -> anyhow::Result<$result> {
                    if let EValue::$type { value } = self {
                        Ok(value)
                    } else {
                        anyhow::bail!(
                            "Invalid cast from {:?} to {}",
                            self,
                            rust_i18n::t!(stringify!($name))
                        )
                    }
                }
                pub fn [<try_as_ $name>](&self) -> anyhow::Result<&$result> {
                    if let EValue::$type { value } = self {
                        Ok(&value)
                    } else {
                        anyhow::bail!(
                            "Invalid cast from {:?} to {}",
                            self,
                            rust_i18n::t!(stringify!($name))
                        )
                    }
                }
            }
        }
    };
}

try_to!(Scalar, ENumber, scalar);
try_to!(Vec2, EVector2, vec2);
try_to!(Boolean, bool, boolean);
try_to!(String, String, string);

impl<'a, T: TryFrom<&'a EValue, Error = anyhow::Error>, A: Array<Item = T>>
    TryFrom<EValueInputWrapper<'a>> for SmallVec<A>
{
    type Error = anyhow::Error;

    fn try_from(value: EValueInputWrapper<'a>) -> Result<Self, Self::Error> {
        value
            .0
            .into_iter()
            .map(T::try_from)
            .collect::<Result<SmallVec<A>, anyhow::Error>>()
    }
}

impl WidgetValueTrait for EValue {
    type Response = EditorGraphResponse;
    type UserState = EditorGraphState;
    type NodeData = EditorNodeData;
    fn value_widget(
        &mut self,
        _param_name: &str,
        _node_id: NodeId,
        _ui: &mut egui::Ui,
        _user_state: &mut EditorGraphState,
        _node_data: &EditorNodeData,
    ) -> Vec<EditorGraphResponse> {
        // This trait is used to tell the library which UI to display for the
        // inline parameter widgets.
        // draw_evalue(self, ui, param_name, &user_state.registry);
        todo!();
        // This allows you to return your responses from the inline widgets.
        // Vec::new()
    }
}

impl Display for EValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EValue::Boolean { value } => write!(f, "{value}"),
            EValue::Scalar { value } => write!(f, "{value}"),
            EValue::Vec2 { value } => write!(f, "{value}"),
            EValue::String { value } => write!(f, "\"{value}\""),
            EValue::Struct { ident, fields } => {
                write!(
                    f,
                    "{ident}{{{}}}",
                    fields
                        .iter()
                        .map(|(field, value)| format!("\"{field}\": {value}"))
                        .join(", ")
                )
            }
            EValue::Enum {
                variant: ident,
                data,
            } => {
                write!(f, "{ident}({data})")
            }
            EValue::Unknown { value } => write!(f, "JSON({value})"),
            EValue::Id { ty, value } => {
                write!(
                    f,
                    "Id<{ty}>({})",
                    value.map(|e| e.raw().as_str()).unwrap_or("null")
                )
            }
            EValue::Ref { ty, value } => {
                write!(
                    f,
                    "Ref<{ty}>({})",
                    value.map(|e| e.raw().as_str()).unwrap_or("null")
                )
            }
        }
    }
}
