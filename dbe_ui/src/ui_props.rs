use dbe_backend::etype::econst::ETypeConst;
use dbe_backend::extra_properties;
use dbe_backend::value::ENumber;
use egui::Color32;
use miette::{bail, Context, IntoDiagnostic};
use ustr::Ustr;

pub struct PinColor(pub Color32);
impl TryFrom<ETypeConst> for PinColor {
    type Error = miette::Error;

    fn try_from(value: ETypeConst) -> Result<Self, Self::Error> {
        if let ETypeConst::String(str) = value {
            let rgba = csscolorparser::parse(str.as_str())
                .into_diagnostic()
                .context("Invalid color")?
                .to_rgba8();

            Ok(PinColor(Color32::from_rgba_unmultiplied(
                rgba[0], rgba[1], rgba[2], rgba[3],
            )))
        } else {
            bail!("Expected a color string, but got {:?}", value)
        }
    }
}

extra_properties! {
    pub prop<field> editor: Ustr;
    pub prop<object> editor: Ustr;
    pub prop<field> kind: ETypeConst;
    pub prop<field> min: ENumber;
    pub prop<field> max: ENumber;
    pub prop<field> logarithmic: bool;
    pub prop<field> multiline: bool;
    pub prop<field> show_file_path: bool;
    pub prop<field> show_field_path: bool;
    pub prop<field> hide_fields: Ustr;

    pub prop<object> kind: ETypeConst;
    pub prop<object> pin_color: PinColor;
    pub prop<object> graph_search_hide: bool;
}
