//! Exact value starts in the already decoded original JSON. This is navigation,
//! not another data admission parser; escaped keys and strings retain positions.
use anyhow::{Context as _, Result};
use std::collections::BTreeMap;
pub(super) fn values(text: &str) -> Result<BTreeMap<String, usize>> {
    let mut cursor = Cursor {
        text,
        offset: 0,
        lines: std::iter::once(0)
            .chain(text.match_indices('\n').map(|(i, _)| i.saturating_add(1)))
            .collect(),
        found: BTreeMap::new(),
    };
    cursor.value("", 0)?;
    cursor.space();
    anyhow::ensure!(
        cursor.offset == text.len(),
        "Unexpected trailing JSON navigation input"
    );
    Ok(cursor.found)
}
struct Cursor<'a> {
    text: &'a str,
    offset: usize,
    lines: Vec<usize>,
    found: BTreeMap<String, usize>,
}
impl Cursor<'_> {
    fn space(&mut self) {
        while self
            .text
            .as_bytes()
            .get(self.offset)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.offset = self.offset.saturating_add(1);
        }
    }
    fn take(&mut self, expected: u8) -> Result<()> {
        self.space();
        anyhow::ensure!(
            self.text.as_bytes().get(self.offset) == Some(&expected),
            "Invalid JSON navigation delimiter"
        );
        self.offset = self.offset.saturating_add(1);
        Ok(())
    }
    fn string(&mut self) -> Result<String> {
        self.space();
        let start = self.offset;
        self.take(b'"')?;
        loop {
            match self
                .text
                .as_bytes()
                .get(self.offset)
                .context("Incomplete JSON navigation string")?
            {
                b'"' => {
                    self.offset = self.offset.saturating_add(1);
                    break;
                }
                b'\\' => self.offset = self.offset.saturating_add(2),
                _ => self.offset = self.offset.saturating_add(1),
            }
        }
        Ok(serde_json::from_str(
            self.text
                .get(start..self.offset)
                .context("JSON navigation UTF-8 span unavailable")?,
        )?)
    }
    fn value(&mut self, path: &str, depth: usize) -> Result<()> {
        self.space();
        anyhow::ensure!(
            depth <= 128 && self.found.len() < 8192,
            "JSON navigation bounds exceeded"
        );
        self.found.insert(
            path.into(),
            self.lines.partition_point(|start| *start <= self.offset),
        );
        match self
            .text
            .as_bytes()
            .get(self.offset)
            .context("Missing JSON navigation value")?
        {
            b'{' => {
                self.offset = self.offset.saturating_add(1);
                self.space();
                if self.text.as_bytes().get(self.offset) != Some(&b'}') {
                    loop {
                        let key = self.string()?.replace('~', "~0").replace('/', "~1");
                        self.take(b':')?;
                        self.value(&format!("{path}/{key}"), depth.saturating_add(1))?;
                        self.space();
                        if self.text.as_bytes().get(self.offset) != Some(&b',') {
                            break;
                        }
                        self.offset = self.offset.saturating_add(1);
                    }
                }
                self.take(b'}')?;
            }
            b'[' => {
                self.offset = self.offset.saturating_add(1);
                self.space();
                let mut index = 0usize;
                if self.text.as_bytes().get(self.offset) != Some(&b']') {
                    loop {
                        self.value(&format!("{path}/{index}"), depth.saturating_add(1))?;
                        index = index.saturating_add(1);
                        self.space();
                        if self.text.as_bytes().get(self.offset) != Some(&b',') {
                            break;
                        }
                        self.offset = self.offset.saturating_add(1);
                    }
                }
                self.take(b']')?;
            }
            b'"' => {
                self.string()?;
            }
            _ => {
                let start = self.offset;
                while self
                    .text
                    .as_bytes()
                    .get(self.offset)
                    .is_some_and(|b| !b.is_ascii_whitespace() && !matches!(*b, b',' | b']' | b'}'))
                {
                    self.offset = self.offset.saturating_add(1);
                }
                anyhow::ensure!(self.offset > start, "Invalid JSON navigation token");
                let _: serde_json::Value = serde_json::from_str(&self.text[start..self.offset])?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn structural_positions_ignore_repeated_names_and_decode_escaped_keys() -> Result<()> {
        let text="{\n \"description\":\"name: property.step, 日本語\\n\\\"quoted\\\"\",\n \"propert\\u0069es\":[\n  {\"name\":\"step\",\"statement\":{\"op\":\"true\"}},\n  {\"name\":\"step\"}\n ],\n \"a/b~c\": [null,true,12,\"x\"]\n}";
        let found = values(text)?;
        assert_eq!(found.get("/properties/0"), Some(&4));
        assert_eq!(found.get("/properties/1"), Some(&5));
        assert_eq!(found.get("/a~1b~0c/3"), Some(&7));
        assert_eq!(
            values("{\"properties\":[{\"name\":\"x\"}]}")?.get("/properties/0"),
            Some(&1)
        );
        for invalid in [
            "",
            "{} x",
            "{\"a\":",
            "[1,]",
            "{\"a\":\"x",
            "{\"a\":true false}",
        ] {
            assert!(values(invalid).is_err(), "{invalid}");
        }
        Ok(())
    }
}
