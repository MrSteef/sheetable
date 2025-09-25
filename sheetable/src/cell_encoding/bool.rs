use std::{convert::Infallible, error::Error, fmt};

use serde_json::Value;

use crate::cell_encoding::{DecodeCell, EncodeCell};

impl EncodeCell for bool {
    type Error = Infallible;
    fn encode_cell(&self) -> Result<Value, Self::Error> {
        Ok(Value::Bool(*self))
    }
}
impl DecodeCell for bool {
    type Error = DecodeBoolError;
    fn decode_cell(v: &Value) -> Result<Self, Self::Error> {
        match v {
            Value::Bool(b) => Ok(*b),
            Value::String(s) => match s.trim().to_ascii_lowercase().as_str() {
                "true" | "1" => Ok(true),
                "false" | "0" => Ok(false),
                _ => Err(DecodeBoolError),
            },
            _ => Err(DecodeBoolError),
        }
    }
}
#[derive(Debug)]
pub struct DecodeBoolError;
impl fmt::Display for DecodeBoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "expected bool or \"true\"/\"false\"/\"1\"/\"0\" string")
    }
}
impl Error for DecodeBoolError {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn bool_encode_decode_roundtrip() {
        let b = true;
        let encoded = b.encode_cell().unwrap();
        assert_eq!(encoded, json!(true));
        let decoded = bool::decode_cell(&json!(true)).unwrap();
        assert!(decoded);

        let decoded_1 = bool::decode_cell(&json!("1")).unwrap();
        assert!(decoded_1);
        let decoded_0 = bool::decode_cell(&json!("0")).unwrap();
        assert!(!decoded_0);

        // Failure cases
        assert!(bool::decode_cell(&json!("maybe")).is_err());
        assert!(bool::decode_cell(&json!(42)).is_err());
    }
}
