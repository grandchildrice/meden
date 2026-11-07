//! AgentDojo's client echoes parsed tool arguments through Python's
//! json.dumps defaults. Match that history representation, including spacing,
//! insertion order and ASCII escaping, instead of echoing the model's raw
//! compact JSON. Execution still validates the original arguments separately.

use std::io;

use serde::Serialize;
use serde_json::ser::{Formatter, Serializer};

pub(crate) fn arguments(raw: &str) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return raw.to_owned();
    };
    let mut bytes = Vec::new();
    if value
        .serialize(&mut Serializer::with_formatter(&mut bytes, PythonJson))
        .is_err()
    {
        return raw.to_owned();
    }
    String::from_utf8(bytes).unwrap_or_else(|_| raw.to_owned())
}

struct PythonJson;
impl Formatter for PythonJson {
    fn begin_array_value<W: io::Write + ?Sized>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> io::Result<()> {
        if first {
            Ok(())
        } else {
            writer.write_all(b", ")
        }
    }
    fn begin_object_key<W: io::Write + ?Sized>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> io::Result<()> {
        if first {
            Ok(())
        } else {
            writer.write_all(b", ")
        }
    }
    fn begin_object_value<W: io::Write + ?Sized>(&mut self, writer: &mut W) -> io::Result<()> {
        writer.write_all(b": ")
    }
    fn write_string_fragment<W: io::Write + ?Sized>(
        &mut self,
        writer: &mut W,
        fragment: &str,
    ) -> io::Result<()> {
        if fragment.is_ascii() {
            return writer.write_all(fragment.as_bytes());
        }
        for character in fragment.chars() {
            if character.is_ascii() {
                let mut bytes = [0; 4];
                writer.write_all(character.encode_utf8(&mut bytes).as_bytes())?;
            } else {
                let mut units = [0; 2];
                for unit in character.encode_utf16(&mut units).iter() {
                    write!(writer, "\\u{unit:04x}")?;
                }
            }
        }
        Ok(())
    }
    fn write_f64<W: io::Write + ?Sized>(&mut self, writer: &mut W, value: f64) -> io::Result<()> {
        let shortest = format!("{value:?}");
        if let Some((base, exponent)) = shortest.split_once('e') {
            if let Ok(exponent) = exponent.parse::<i32>() {
                return write!(writer, "{base}e{exponent:+03}");
            }
        }
        writer.write_all(shortest.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compact_unicode_and_nested_arguments_match_python_history() {
        assert_eq!(
            arguments(r#"{"z":1e-5,"a":1e20,"s":"é🙂","b":[true,null,{"x":"\n"}]}"#),
            r#"{"z": 1e-05, "a": 1e+20, "s": "\u00e9\ud83d\ude42", "b": [true, null, {"x": "\n"}]}"#
        );
    }
    #[test]
    fn invalid_arguments_remain_available_for_error_feedback() {
        assert_eq!(arguments("not JSON"), "not JSON");
    }
}
