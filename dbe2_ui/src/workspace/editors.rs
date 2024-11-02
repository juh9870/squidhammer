use crate::m_try;
use crate::workspace::editors::boolean::BooleanEditor;
use crate::workspace::editors::consts::ConstEditor;
use crate::workspace::editors::enums::EnumEditor;
use crate::workspace::editors::errors::{ErrorEditor, ErrorProps};
use crate::workspace::editors::number::NumberEditor;
use crate::workspace::editors::rgb::RgbEditor;
use crate::workspace::editors::string::StringEditor;
use crate::workspace::editors::structs::StructEditor;
use crate::workspace::editors::utils::{prop_opt, EditorSize};
use crate::workspace::editors::wrapped::WrappedEditor;
use ahash::AHashMap;
use dbe2::diagnostic::context::DiagnosticContextRef;
use dbe2::etype::econst::ETypeConst;
use dbe2::etype::eitem::EItemInfo;
use dbe2::etype::EDataType;
use dbe2::registry::{EObjectType, ETypesRegistry};
use dbe2::value::EValue;
use downcast_rs::{impl_downcast, Downcast};
use dyn_clone::DynClone;
use egui::Ui;
use list::ListEditor;
use miette::{bail, miette};
use std::fmt::Debug;
use std::ops::Deref;
use std::sync::LazyLock;
use ustr::{Ustr, UstrMap};

mod utils;

mod boolean;
mod consts;
mod enums;
mod errors;
mod list;
mod number;
mod rgb;
mod string;
mod structs;
mod wrapped;

static EDITORS: LazyLock<UstrMap<Box<dyn Editor>>> = LazyLock::new(|| default_editors().collect());

fn default_editors() -> impl Iterator<Item = (Ustr, Box<dyn Editor>)> {
    let v: Vec<(Ustr, Box<dyn Editor>)> = vec![
        ("number".into(), Box::new(NumberEditor::new(false))),
        ("slider".into(), Box::new(NumberEditor::new(true))),
        ("string".into(), Box::new(StringEditor)),
        ("boolean".into(), Box::new(BooleanEditor)),
        ("struct".into(), Box::new(StructEditor)),
        ("rgba".into(), Box::new(RgbEditor::new(true))),
        ("rgb".into(), Box::new(RgbEditor::new(false))),
        ("const".into(), Box::new(ConstEditor)),
        ("enum".into(), Box::new(EnumEditor)),
        ("list".into(), Box::new(ListEditor)),
        // TODO: proper editors for ids
        (
            "ids/numeric".into(),
            Box::new(WrappedEditor::new(NumberEditor::new(false), "id".into())),
        ),
        (
            "ref/numeric".into(),
            Box::new(WrappedEditor::new(NumberEditor::new(false), "id".into())),
        ),
        // Enums
        // (
        //     "enum".to_string(),
        //     Box::new(EnumEditorConstructor::from(EnumEditorType::Auto)),
        // ),
        // (
        //     "enum:toggle".to_string(),
        //     Box::new(EnumEditorConstructor::from(EnumEditorType::Toggle)),
        // ),
        // (
        //     "enum:full".to_string(),
        //     Box::new(EnumEditorConstructor::from(EnumEditorType::Full)),
        // ),
        // ("const".to_string(), Box::new(ConstEditorConstructor)),
        // ("id".to_string(), Box::new(ObjectIdEditorConstructor)),
        // // other
        // ("rgb".to_string(), Box::new(RgbEditorConstructor::rgb())),
        // ("rgba".to_string(), Box::new(RgbEditorConstructor::rgba())),
    ];
    v.into_iter()
}
type Props<'a> = &'a AHashMap<String, ETypeConst>;

trait EditorProps: std::any::Any + DynClone + Downcast {
    fn pack(self) -> DynProps
    where
        Self: Sized,
    {
        Some(Box::new(self))
    }
}

impl_downcast!(EditorProps);

fn cast_props<T: EditorProps>(props: &DynProps) -> &T {
    props.as_ref().and_then(|b| b.downcast_ref::<T>()).unwrap()
}

type DynProps = Option<Box<dyn EditorProps>>;

trait Editor: std::any::Any + Send + Sync + Debug {
    fn props(&self, _reg: &ETypesRegistry, _item: Option<&EItemInfo>) -> miette::Result<DynProps> {
        Ok(None)
    }

    fn size(&self, props: &DynProps) -> EditorSize;

    fn edit(
        &self,
        ui: &mut Ui,
        reg: &ETypesRegistry,
        diagnostics: DiagnosticContextRef,
        field_name: &str,
        value: &mut EValue,
        props: &DynProps,
    ) -> EditorResponse;
}

pub struct EditorData(&'static dyn Editor, DynProps);

#[derive(Debug, Clone)]
pub struct EditorResponse {
    pub changed: bool,
}

impl EditorResponse {
    pub fn new(changed: bool) -> Self {
        Self { changed }
    }

    pub fn unchanged() -> Self {
        Self { changed: false }
    }
}

pub fn editor_for_value(reg: &ETypesRegistry, value: &EValue) -> EditorData {
    editor_for_type(reg, &value.ty())
}

pub fn editor_for_type(reg: &ETypesRegistry, ty: &EDataType) -> EditorData {
    m_try(|| {
        let editor = editor_for_raw(reg, ty, None)?;

        Ok(EditorData(editor, editor.props(reg, None)?))
    })
    .unwrap_or_else(|err| EditorData(&ErrorEditor, ErrorProps(err.to_string()).pack()))
}

pub fn editor_for_item(reg: &ETypesRegistry, item: &EItemInfo) -> EditorData {
    m_try(|| {
        let name = prop_opt::<Ustr>(item.extra_properties(), "editor")?;

        let editor = editor_for_raw(reg, &item.ty(), name)?;

        Ok(EditorData(editor, editor.props(reg, Some(item))?))
    })
    .unwrap_or_else(|err| EditorData(&ErrorEditor, ErrorProps(err.to_string()).pack()))
}

fn editor_for_raw(
    reg: &ETypesRegistry,
    ty: &EDataType,
    name: Option<Ustr>,
) -> miette::Result<&'static dyn Editor> {
    let name = match name {
        None => match ty {
            EDataType::Number => "number".into(),
            EDataType::String => "string".into(),
            EDataType::Boolean => "boolean".into(),
            EDataType::Const { .. } => "const".into(),
            EDataType::Object { ident } => {
                let data = reg
                    .get_object(ident)
                    .ok_or_else(|| miette!("Unknown object ID `{}`", ident))?;
                if let Some(prop) = data.extra_properties().get("editor") {
                    Ustr::try_from(*prop).map_err(|e| {
                        miette!(
                            "Bad value for property `editor` in object `{}`: {}",
                            ident,
                            e
                        )
                    })?
                } else {
                    match data {
                        EObjectType::Struct(_) => "struct".into(),
                        EObjectType::Enum(_) => "enum".into(),
                    }
                }
            }
            EDataType::List { .. } => "list".into(),
            EDataType::Map { .. } => "map".into(),
        },
        Some(name) => name,
    };

    let Some(editor) = EDITORS.get(&name) else {
        bail!("unknown editor `{}`", name)
    };

    Ok(editor.deref())
}

impl EditorData {
    pub fn show(
        &self,
        ui: &mut Ui,
        reg: &ETypesRegistry,
        diagnostics: DiagnosticContextRef,
        field_name: &str,
        value: &mut EValue,
    ) -> EditorResponse {
        let Self(editor, props) = self;
        editor.edit(ui, reg, diagnostics, field_name, value, props)
    }

    pub fn size(&self) -> EditorSize {
        self.0.size(&self.1)
    }
}
