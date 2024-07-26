use crate::json_utils::repr::JsonRepr;
use crate::json_utils::{json_expected, JsonValue};
use crate::registry::ETypesRegistry;
use utils::color_format::ecolor::Rgba;
use utils::color_format::ColorFormat;

#[derive(Debug)]
pub struct ColorStringRepr {
    alpha_repr: ColorFormat,
}

impl ColorStringRepr {
    pub const ARGB: ColorStringRepr = ColorStringRepr {
        alpha_repr: ColorFormat::argb(),
    };

    pub const RGBA: ColorStringRepr = ColorStringRepr {
        alpha_repr: ColorFormat::argb(),
    };
}

impl JsonRepr for ColorStringRepr {
    fn from_repr(&self, _registry: &ETypesRegistry, data: JsonValue) -> miette::Result<JsonValue> {
        let str = json_expected(data.as_str(), &data, "color")?;

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
        let str = json_expected(data.as_object(), &data, "object")?;

        let a = str.get("a").map(|a| a.as_f64().unwrap()).unwrap_or(1.0) as f32;
        let r = str.get("r").map(|a| a.as_f64().unwrap()).unwrap_or(0.0) as f32;
        let g = str.get("g").map(|a| a.as_f64().unwrap()).unwrap_or(0.0) as f32;
        let b = str.get("b").map(|a| a.as_f64().unwrap()).unwrap_or(0.0) as f32;

        let str = if a == 1.0 {
            ColorFormat::rgb().format(Rgba::from_rgb(r, g, b))
        } else {
            self.alpha_repr
                .format(Rgba::from_rgba_unmultiplied(r, g, b, a))
        };

        Ok(str.into())
    }
}
