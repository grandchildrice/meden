//! A display/transport representation of an original positive u64, never a
//! ticket, approval or evidence receipt. Authority still uses the original value.
use anyhow::Result;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;

/// Canonical decimal string without precision loss in a browser or JSON gateway.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sequence(u64);
impl Sequence {
    /// Observe a positive original counter without replacing its value.
    pub fn new(value: u64) -> Result<Self> {
        anyhow::ensure!(value > 0, "A native sequence must be positive");
        Ok(Self(value))
    }
    /// Exact original integer for the existing host/controller comparison.
    pub fn value(self) -> u64 {
        self.0
    }
}
impl From<std::num::NonZeroU64> for Sequence {
    fn from(value: std::num::NonZeroU64) -> Self {
        Self(value.get())
    }
}
impl Serialize for Sequence {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}
impl<'de> Deserialize<'de> for Sequence {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        let value = text.parse::<u64>().map_err(serde::de::Error::custom)?;
        if value == 0 || value.to_string() != text {
            return Err(serde::de::Error::custom(
                "Native sequences require canonical positive u64 decimal strings",
            ));
        }
        Ok(Self(value))
    }
}
impl JsonSchema for Sequence {
    fn schema_name() -> Cow<'static, str> {
        "NativeSequence".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        schemars::json_schema!({"type":"string","pattern":"^[1-9][0-9]{0,19}$","maxLength":20,"format":"meden-positive-u64"})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    proptest! {
        #[test]
        fn every_positive_original_round_trips(value in 1..=u64::MAX) {
            let observed = Sequence::new(value).map_err(|error| TestCaseError::fail(error.to_string()))?;
            let encoded = serde_json::to_string(&observed).map_err(|error| TestCaseError::fail(error.to_string()))?;
            let decoded: Sequence = serde_json::from_str(&encoded).map_err(|error| TestCaseError::fail(error.to_string()))?;
            prop_assert_eq!(decoded.value(), value);
            prop_assert_eq!(encoded, format!("\"{value}\""));
        }
    }
    #[test]
    fn noncanonical_numbers_and_out_of_range_values_never_decode() -> Result<()> {
        assert!(Sequence::new(0).is_err());
        for text in [
            "",
            "0",
            "01",
            "+1",
            "-1",
            " 1",
            "1 ",
            "1.0",
            "1e1",
            "18446744073709551616",
            "１２",
            "🧪",
        ] {
            assert!(serde_json::from_value::<Sequence>(serde_json::json!(text)).is_err());
        }
        for value in [
            serde_json::json!(1),
            serde_json::json!(null),
            serde_json::json!({"value":"1"}),
        ] {
            assert!(serde_json::from_value::<Sequence>(value).is_err());
        }
        assert_eq!(
            serde_json::from_str::<Sequence>("\"18446744073709551615\"")?.value(),
            u64::MAX
        );
        Ok(())
    }
}
