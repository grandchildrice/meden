//! Rebase backend-local schema references without merging their vocabularies.
use schemars::{
    transform::{transform_subschemas, Transform},
    Schema,
};
use serde_json::Value;
use std::io;

pub(super) fn namespace(id: &str, value: Value) -> io::Result<Value> {
    struct Rebase {
        root: String,
        invalid: bool,
    }
    impl Transform for Rebase {
        fn transform(&mut self, schema: &mut Schema) {
            if ["$id", "$anchor", "$dynamicAnchor", "$dynamicRef"]
                .iter()
                .any(|key| schema.get(*key).is_some())
            {
                self.invalid = true;
            }
            if let Some(reference) = schema.get_mut("$ref") {
                if let Some(original) = reference.as_str() {
                    if original == "#" || original.starts_with("#/") {
                        *reference = Value::String(format!(
                            "{}{}",
                            self.root,
                            original.strip_prefix('#').unwrap_or_default()
                        ));
                    } else {
                        self.invalid = true;
                    }
                } else {
                    self.invalid = true;
                }
            }
            // The library visits schema positions, excluding const/default
            // values and property names that happen to look like keywords.
            transform_subschemas(self, schema);
        }
    }
    let mut schema = Schema::try_from(value).map_err(io::Error::other)?;
    schema.remove("$schema");
    schema.remove("title");
    let mut rebase = Rebase {
        root: format!("#/$defs/{id}"),
        invalid: false,
    };
    rebase.transform(&mut schema);
    if rebase.invalid {
        return Err(io::Error::other(
            "Registered schema requires unsupported external or anchored references",
        ));
    }
    Ok(schema.to_value())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recursive_local_schemas_keep_distinct_names_and_literal_data() -> io::Result<()> {
        let original = serde_json::json!({"$defs":{"Word":{"anyOf":[{"type":"integer"},{"$ref":"#/$defs/Word"}]}},"properties":{"word":{"$ref":"#/$defs/Word"},"$ref":{"type":"string"}},"default":{"$ref":"literal-data"}});
        let first = namespace("first", original.clone())?;
        let second = namespace("second", original)?;
        assert_eq!(
            first
                .pointer("/properties/word/$ref")
                .and_then(Value::as_str),
            Some("#/$defs/first/$defs/Word")
        );
        assert_eq!(
            second
                .pointer("/$defs/Word/anyOf/1/$ref")
                .and_then(Value::as_str),
            Some("#/$defs/second/$defs/Word")
        );
        assert_eq!(
            first.pointer("/default/$ref").and_then(Value::as_str),
            Some("literal-data")
        );
        assert_eq!(
            first
                .pointer("/properties/$ref/type")
                .and_then(Value::as_str),
            Some("string")
        );
        assert!(namespace(
            "invalid",
            serde_json::json!({"$ref":"https://example.test/schema"})
        )
        .is_err());
        assert!(namespace("invalid", serde_json::json!({"$anchor":"local"})).is_err());
        Ok(())
    }
}
