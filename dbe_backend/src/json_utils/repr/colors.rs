use crate::etype::eenum::pattern::EnumPattern;
use crate::json_utils::repr::JsonRepr;
use crate::json_utils::{json_expected, JsonValue};
use crate::registry::ETypesRegistry;
use utils::color_format::ecolor::Rgba;
use utils::color_format::ColorFormat;

#[derive(Debug)]
pub struct ColorStringRepr {
    id: &'static str,
    alpha_repr: ColorFormat,
}

impl ColorStringRepr {
    pub const ARGB: ColorStringRepr = ColorStringRepr {
        id: "argb",
        alpha_repr: ColorFormat::argb(),
    };

    pub const RGBA: ColorStringRepr = ColorStringRepr {
        id: "rgba",
        alpha_repr: ColorFormat::argb(),
    };
}

impl JsonRepr for ColorStringRepr {
    fn id(&self) -> &'static str {
        self.id
    }

    fn from_repr(
        &self,
        _registry: &ETypesRegistry,
        data: &mut JsonValue,
        _ignore_extra_fields: bool,
    ) -> miette::Result<JsonValue> {
        let str = json_expected(data.as_str(), data, "color")?;

        let color = if let Ok(color) = ColorFormat::rgb().parse(str) {
            color
        } else {
            self.alpha_repr.parse(str)?
        };

        let mut fields = serde_json::value::Map::new();
        let c = color.to_rgba_unmultiplied();
        fields.insert("r".to_string(), c[0].into());
        fields.insert("g".to_string(), c[1].into());
        fields.insert("b".to_string(), c[2].into());
        fields.insert("a".to_string(), c[3].into());

        Ok(fields.into())
    }

    fn into_repr(&self, _registry: &ETypesRegistry, data: JsonValue) -> miette::Result<JsonValue> {
        let obj = json_expected(data.as_object(), &data, "object")?;

        let a = obj.get("a").map_or(1.0, |a| a.as_f64().unwrap()) as f32;
        let r = obj.get("r").map_or(0.0, |a| a.as_f64().unwrap()) as f32;
        let g = obj.get("g").map_or(0.0, |a| a.as_f64().unwrap()) as f32;
        let b = obj.get("b").map_or(0.0, |a| a.as_f64().unwrap()) as f32;

        let str = if a == 1.0 {
            ColorFormat::rgb().format(Rgba::from_rgb(r, g, b))
        } else {
            self.alpha_repr
                .format(Rgba::from_rgba_unmultiplied(r, g, b, a))
        };

        Ok(str.into())
    }

    fn enum_pat(&self) -> Option<EnumPattern> {
        Some(EnumPattern::String)
    }
}
