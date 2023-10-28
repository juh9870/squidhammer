use crate::graph::nodes::data::EditorNodeData;
use crate::value::etype::registry::eenum::EEnumVariantId;
use crate::value::etype::registry::{ETypeId, EValueId};
use crate::EditorGraphState;

use egui_node_graph::{NodeId, WidgetValueTrait};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use smallvec::{Array, SmallVec};
use std::fmt::{Display, Formatter};
use ustr::UstrMap;

use crate::graph::event::EditorGraphResponse;
use crate::value::draw::editor::EFieldEditorError;
use crate::value::etype::registry::eitem::EItemType;
use crate::value::etype::{EDataType, ETypeConst};
pub use serde_json::Value as JsonValue;
use tracing::trace;

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
/// with a DataType of Number and a ValueType of String.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EValue {
    Null,
    Boolean {
        value: bool,
    },
    Number {
        value: ENumber,
    },
    String {
        value: String,
    },
    Struct {
        ident: ETypeId,
        fields: UstrMap<EValue>,
    },
    Id {
        ty: ETypeId,
        value: Option<EValueId>,
    },
    Ref {
        ty: ETypeId,
        value: Option<EValueId>,
    },
    Enum {
        variant: EEnumVariantId,
        data: Box<EValue>,
    },
}

impl EValue {
    pub fn ty(&self) -> EDataType {
        match self {
            EValue::Null => EDataType::Const {
                value: ETypeConst::Null,
            },
            EValue::Boolean { .. } => EDataType::Boolean,
            EValue::Number { .. } => EDataType::Number,
            EValue::String { .. } => EDataType::String,
            EValue::Struct { ident, .. } => EDataType::Object { ident: *ident },
            EValue::Enum { variant, .. } => EDataType::Object {
                ident: variant.enum_id(),
            },
            EValue::Id { ty, .. } => EDataType::Id { ty: *ty },
            EValue::Ref { ty, .. } => EDataType::Ref { ty: *ty },
        }
    }
}

impl Default for EValue {
    fn default() -> Self {
        // NOTE: This is just a dummy `Default` implementation. The library
        // requires it to circumvent some internal borrow checker issues.
        Self::Number { value: 0.0 }
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

            impl <'a> TryFrom<&'a EValue> for &'a $result {
                type Error = anyhow::Error;

                fn try_from(value: &'a EValue) -> Result<Self, Self::Error> {
                    value.[<try_as_ $name>]()
                }
            }

            impl <'a> TryFrom<&'a mut EValue> for &'a mut $result {
                type Error = anyhow::Error;

                fn try_from(value: &'a mut EValue) -> Result<Self, Self::Error> {
                    value.[<try_as_ $name _mut>]()
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

                pub fn [<try_as_ $name _mut>](&mut self) -> anyhow::Result<&mut $result> {
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
            }
        }
    };
}

try_to!(Number, ENumber, number);
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
        param_name: &str,
        node_id: NodeId,
        ui: &mut egui::Ui,
        user_state: &mut EditorGraphState,
        node_data: &EditorNodeData,
    ) -> Vec<EditorGraphResponse> {
        let mut commands = vec![];
        let reg = user_state.registry.borrow();
        let editor = match node_data.editors.get(param_name) {
            None => {
                let editor = match reg.editor_for(None, &EItemType::default_item_for(self)) {
                    Ok(editor) => editor,
                    Err(err) => Box::new(EFieldEditorError::new(err.to_string(), self.ty())),
                };
                trace!(?node_id, param_name, ?editor, "New editor is requested");
                commands.push(editor);
                &commands[0]
            }
            Some(editor) => editor,
        };

        editor.draw(ui, &reg, param_name, self);

        commands
            .into_iter()
            .map(|e| EditorGraphResponse::ChangeEditor {
                node_id,
                editor: e,
                field: param_name.to_string(),
            })
            .collect_vec()
    }
}

impl Display for EValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EValue::Boolean { value } => write!(f, "{value}"),
            EValue::Number { value } => write!(f, "{value}"),
            EValue::String { value } => write!(f, "\"{value}\""),
            EValue::Null => write!(f, "null"),
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
            EValue::Id { ty, value } => {
                write!(
                    f,
                    "Id<{ty}>({})",
                    value
                        .map(|e| e.to_string())
                        .unwrap_or_else(|| "null".to_string())
                )
            }
            EValue::Ref { ty, value } => {
                write!(
                    f,
                    "Ref<{ty}>({})",
                    value
                        .map(|e| e.to_string())
                        .unwrap_or_else(|| "null".to_string())
                )
            }
        }
    }
}
