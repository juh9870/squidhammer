use anyhow::{anyhow, bail};
use egui::Rgba;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Debug, Copy, Clone, Default, Eq, PartialEq, Hash)]
pub enum ColorChannel {
    #[default]
    None,
    Red,
    Green,
    Blue,
    Alpha,
}

impl Display for ColorChannel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ColorChannel::None => write!(f, ""),
            ColorChannel::Red => write!(f, "R"),
            ColorChannel::Green => write!(f, "G"),
            ColorChannel::Blue => write!(f, "B"),
            ColorChannel::Alpha => write!(f, "A"),
        }
    }
}

impl TryFrom<char> for ColorChannel {
    type Error = anyhow::Error;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        Ok(match value {
            'R' | 'r' => ColorChannel::Red,
            'G' | 'g' => ColorChannel::Green,
            'B' | 'b' => ColorChannel::Blue,
            'A' | 'a' => ColorChannel::Alpha,
            _ => bail!("Invalid color component: `{value}`"),
        })
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ColorFormat([ColorChannel; 4]);

impl ColorFormat {
    pub const fn rgb() -> Self {
        ColorFormat([
            ColorChannel::Red,
            ColorChannel::Green,
            ColorChannel::Blue,
            ColorChannel::None,
        ])
    }

    pub const fn rgba() -> Self {
        ColorFormat([
            ColorChannel::Red,
            ColorChannel::Green,
            ColorChannel::Blue,
            ColorChannel::Alpha,
        ])
    }

    pub const fn argb() -> Self {
        ColorFormat([
            ColorChannel::Alpha,
            ColorChannel::Red,
            ColorChannel::Green,
            ColorChannel::Blue,
        ])
    }

    pub fn with_alpha(&self) -> bool {
        self.0[3] != ColorChannel::None
    }

    pub fn parse(&self, mut color: &str) -> anyhow::Result<Rgba> {
        if color.starts_with('#') {
            color = &color[1..];
        }
        let components = if color.len() == 6 {
            [&color[0..2], &color[2..4], &color[4..6], &color[6..6]]
        } else if color.len() == 8 {
            [&color[0..2], &color[2..4], &color[4..6], &color[6..8]]
        } else {
            bail!(
                "Invalid color length for format {}",
                self.to_string().to_ascii_uppercase()
            )
        };
        let mut red = 0.0;
        let mut green = 0.0;
        let mut blue = 0.0;
        let mut alpha = 1.0;

        for (i, (channel, raw)) in self.0.iter().zip(components).enumerate() {
            if raw.is_empty()
                && i == 3
                && (channel == &ColorChannel::None || channel == &ColorChannel::Alpha)
            {
                continue;
            }
            let value = u8::from_str_radix(raw, 16)
                .map_err(|_| anyhow!("Failed to parse {} color component: {}", channel, raw))?
                as f32;
            match channel {
                ColorChannel::None => {
                    bail!("Too many color components for format {self}")
                }
                ColorChannel::Red => red = value / 255.0,
                ColorChannel::Green => green = value / 255.0,
                ColorChannel::Blue => blue = value / 255.0,
                ColorChannel::Alpha => alpha = value / 255.0,
            }
        }

        Ok(Rgba::from_rgba_premultiplied(red, green, blue, alpha))
    }

    pub fn format(&self, color: Rgba) -> String {
        let mut builder = "#".to_string();

        let num = |x: f32| format!("{:0>2X}", ((x * 255.0) as u8));

        for channel in self.0 {
            match channel {
                ColorChannel::None => break,
                ColorChannel::Red => builder.push_str(&num(color.r())),
                ColorChannel::Green => builder.push_str(&num(color.g())),
                ColorChannel::Blue => builder.push_str(&num(color.b())),
                ColorChannel::Alpha => builder.push_str(&num(color.a())),
            }
        }

        builder
    }

    pub fn channels(&self) -> impl Iterator<Item = &ColorChannel> {
        self.0.iter().take(if self.with_alpha() { 4 } else { 3 })
    }
}

impl Display for ColorFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}{}{}", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

impl FromStr for ColorFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let format_error = |i| anyhow!("Color format must have 3 or 4 components, got {i}");

        let mut components = s.chars().map(TryInto::<ColorChannel>::try_into);

        let arr = [
            components.next().ok_or_else(|| format_error(0))??,
            components.next().ok_or_else(|| format_error(1))??,
            components.next().ok_or_else(|| format_error(2))??,
            components
                .next()
                .unwrap_or_else(|| Ok(ColorChannel::None))?,
        ];

        let rest_count = components.count();
        if rest_count > 0 {
            return Err(format_error(rest_count + 4));
        }

        if arr[3] == ColorChannel::None
            && (arr[0] == ColorChannel::Alpha
                || arr[1] == ColorChannel::Alpha
                || arr[2] == ColorChannel::Alpha)
        {
            bail!("Color format must include Red, Green, and Blue channels")
        }

        if arr[0] == arr[1]
            || arr[0] == arr[2]
            || arr[0] == arr[3]
            || arr[1] == arr[2]
            || arr[1] == arr[3]
            || arr[2] == arr[3]
        {
            bail!("All color format channels must be unique",)
        }
        Ok(ColorFormat(arr))
    }
}

#[cfg(test)]
mod tests {
    use super::{ColorChannel, ColorFormat};
    use egui::Rgba;
    use rstest::rstest;
    use std::str::FromStr;

    const R: ColorChannel = ColorChannel::Red;
    const G: ColorChannel = ColorChannel::Green;
    const B: ColorChannel = ColorChannel::Blue;
    const A: ColorChannel = ColorChannel::Alpha;
    const N: ColorChannel = ColorChannel::None;

    #[rstest]
    #[case("RGBA", ColorFormat([R, G, B, A]))]
    #[case("ARGB", ColorFormat([A, R, G, B]))]
    #[case("RGB", ColorFormat([R, G, B, N]))]
    #[case("BGR", ColorFormat([B, G, R, N]))]
    #[case("rgba", ColorFormat([R, G, B, A]))]
    #[case("rBaG", ColorFormat([R, B, A, G]))]
    fn should_parse_formats(#[case] raw: String, #[case] format: ColorFormat) {
        let parsed = ColorFormat::from_str(&raw).expect("Should parse");
        assert_eq!(parsed, format)
    }

    #[rstest]
    #[case("RGB-")]
    #[case("rgfA")]
    #[case("++++")]
    fn should_fail_on_bad_symbols(#[case] raw: String) {
        assert!(ColorFormat::from_str(&raw).is_err())
    }

    #[rstest]
    #[case("RG")]
    #[case("b")]
    #[case("")]
    #[case("RGBABA")]
    fn should_fail_on_bad_length(#[case] raw: String) {
        assert!(ColorFormat::from_str(&raw).is_err())
    }

    #[rstest]
    #[case("RGR")]
    #[case("ARGA")]
    #[case("RRR")]
    #[case("BBBB")]
    fn should_fail_on_duplicates(#[case] raw: String) {
        assert!(ColorFormat::from_str(&raw).is_err())
    }

    #[rstest]
    #[case("RGA")]
    #[case("RAB")]
    #[case("AGB")]
    fn should_fail_on_missing_color_channel(#[case] raw: String) {
        assert!(ColorFormat::from_str(&raw).is_err())
    }

    fn parse_color(raw: &str, format: &str) -> anyhow::Result<Rgba> {
        let format = ColorFormat::from_str(format).expect("Should parse");
        format.parse(raw)
    }

    #[rstest]
    #[case("#ffffff", "rgb", Rgba::from_rgba_premultiplied(1.0, 1.0, 1.0, 1.0))]
    #[case("#ffffff00", "rgba", Rgba::from_rgba_premultiplied(1.0, 1.0, 1.0, 0.0))]
    #[case("#000000", "rgba", Rgba::from_rgba_premultiplied(0.0, 0.0, 0.0, 1.0))]
    #[case("#800000FF", "arbg", Rgba::from_rgba_premultiplied(0.0, 1.0, 0.0, 128.0 / 255.0))]
    fn should_parse_color(#[case] raw: String, #[case] format: String, #[case] expected: Rgba) {
        let parsed = parse_color(&raw, &format).expect("Should parse color");
        assert_eq!(parsed, expected);
        let parsed = parse_color(&raw[1..], &format).expect("Should parse color without #");
        assert_eq!(parsed, expected, "Without #");
    }

    #[rstest]
    #[case("#ffgghh", "rgb")]
    #[case("#  --++**", "rgba")]
    #[case("#aabbjj", "rgb")]
    fn should_fail_to_parse_color_bad_symbols(#[case] color: String, #[case] format: String) {
        assert!(parse_color(&color, &format).is_err());
        assert!(parse_color(&color[1..], &format).is_err(), "Without #");
    }

    #[rstest]
    #[case("#00112233", "rgb")]
    #[case("#ffddaa", "argb")]
    #[case("#fff", "rgb")]
    #[case("#ffaa1", "rgb")]
    fn should_fail_to_parse_color_bad_length(#[case] color: String, #[case] format: String) {
        assert!(parse_color(&color, &format).is_err());
        assert!(parse_color(&color[1..], &format).is_err(), "Without #");
    }

    #[rstest]
    #[case("rgba", Rgba::from_rgba_premultiplied(1.0, 0.0, 1.0, 1.0), "#FF00FFFF")]
    #[case("argb", Rgba::from_rgba_premultiplied(1.0, 0.0, 1.0, 0.0), "#00FF00FF")]
    #[case("rgb", Rgba::from_rgba_premultiplied(1.0, 0.0, 1.0, 1.0), "#FF00FF")]
    #[case("brg", Rgba::from_rgba_premultiplied(1.0, 0.0, 1.0, 1.0), "#FFFF00")]
    fn should_stringify(#[case] format: String, #[case] color: Rgba, #[case] expected: String) {
        let format = ColorFormat::from_str(&format).expect("Should parse color format");
        assert_eq!(format.format(color), expected);
    }
}
