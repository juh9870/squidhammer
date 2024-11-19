use serde_json::ser::{CharEscape, Formatter, PrettyFormatter};
use std::io::Write;

pub struct DBEJsonFormatter<T: Formatter> {
    parent: T,
}

impl<'a> DBEJsonFormatter<PrettyFormatter<'a>> {
    pub fn pretty() -> Self {
        DBEJsonFormatter::new(PrettyFormatter::new())
    }

    pub fn with_indent(indent: &'a [u8]) -> Self {
        DBEJsonFormatter::new(PrettyFormatter::with_indent(indent))
    }
}

impl<T: Formatter> DBEJsonFormatter<T> {
    pub fn new(parent: T) -> Self {
        Self { parent }
    }
}

impl<T: Formatter> Formatter for DBEJsonFormatter<T> {
    fn write_null<W>(&mut self, writer: &mut W) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.write_null(writer)
    }

    fn write_bool<W>(&mut self, writer: &mut W, value: bool) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.write_bool(writer, value)
    }

    fn write_i8<W>(&mut self, writer: &mut W, value: i8) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.write_i8(writer, value)
    }

    fn write_i16<W>(&mut self, writer: &mut W, value: i16) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.write_i16(writer, value)
    }

    fn write_i32<W>(&mut self, writer: &mut W, value: i32) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.write_i32(writer, value)
    }

    fn write_i64<W>(&mut self, writer: &mut W, value: i64) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.write_i64(writer, value)
    }

    fn write_i128<W>(&mut self, writer: &mut W, value: i128) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.write_i128(writer, value)
    }

    fn write_u8<W>(&mut self, writer: &mut W, value: u8) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.write_u8(writer, value)
    }

    fn write_u16<W>(&mut self, writer: &mut W, value: u16) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.write_u16(writer, value)
    }

    fn write_u32<W>(&mut self, writer: &mut W, value: u32) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.write_u32(writer, value)
    }

    fn write_u64<W>(&mut self, writer: &mut W, value: u64) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.write_u64(writer, value)
    }

    fn write_u128<W>(&mut self, writer: &mut W, value: u128) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.write_u128(writer, value)
    }

    fn write_f32<W>(&mut self, writer: &mut W, value: f32) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        if value.trunc() == value {
            if value <= u32::MAX as f32 && value >= 0.0 {
                return self.parent.write_u32(writer, value as u32);
            } else if value <= i32::MAX as f32 && value >= i32::MIN as f32 {
                return self.parent.write_i32(writer, value as i32);
            }
        }
        self.parent.write_f32(writer, value)
    }

    fn write_f64<W>(&mut self, writer: &mut W, value: f64) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        if value.trunc() == value {
            if value <= u64::MAX as f64 && value >= 0.0 {
                return self.parent.write_u64(writer, value as u64);
            } else if value <= i64::MAX as f64 && value >= i64::MIN as f64 {
                return self.parent.write_i64(writer, value as i64);
            }
        }
        self.parent.write_f64(writer, value)
    }

    fn write_number_str<W>(&mut self, writer: &mut W, value: &str) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.write_number_str(writer, value)
    }

    fn begin_string<W>(&mut self, writer: &mut W) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.begin_string(writer)
    }

    fn end_string<W>(&mut self, writer: &mut W) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.end_string(writer)
    }

    fn write_string_fragment<W>(&mut self, writer: &mut W, fragment: &str) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.write_string_fragment(writer, fragment)
    }

    fn write_char_escape<W>(
        &mut self,
        writer: &mut W,
        char_escape: CharEscape,
    ) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.write_char_escape(writer, char_escape)
    }

    fn write_byte_array<W>(&mut self, writer: &mut W, value: &[u8]) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.write_byte_array(writer, value)
    }

    fn begin_array<W>(&mut self, writer: &mut W) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.begin_array(writer)
    }

    fn end_array<W>(&mut self, writer: &mut W) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.end_array(writer)
    }

    fn begin_array_value<W>(&mut self, writer: &mut W, first: bool) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.begin_array_value(writer, first)
    }

    fn end_array_value<W>(&mut self, writer: &mut W) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.end_array_value(writer)
    }

    fn begin_object<W>(&mut self, writer: &mut W) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.begin_object(writer)
    }

    fn end_object<W>(&mut self, writer: &mut W) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.end_object(writer)
    }

    fn begin_object_key<W>(&mut self, writer: &mut W, first: bool) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.begin_object_key(writer, first)
    }

    fn end_object_key<W>(&mut self, writer: &mut W) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.end_object_key(writer)
    }

    fn begin_object_value<W>(&mut self, writer: &mut W) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.begin_object_value(writer)
    }

    fn end_object_value<W>(&mut self, writer: &mut W) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.end_object_value(writer)
    }

    fn write_raw_fragment<W>(&mut self, writer: &mut W, fragment: &str) -> std::io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.parent.write_raw_fragment(writer, fragment)
    }
}
