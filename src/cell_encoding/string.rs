use std::{convert::Infallible, error::Error, fmt};

use serde_json::Value;

use crate::cell_encoding::{DecodeCell, EncodeCell};

impl EncodeCell for String {
    type Error = Infallible;
    fn encode_cell(&self) -> Result<Value, Self::Error> {
        Ok(Value::String(self.clone()))
    }
}
impl DecodeCell for String {
    type Error = DecodeStringError;
    fn decode_cell(v: &Value) -> Result<Self, Self::Error> {
        match v {
            Value::String(s) => Ok(s.clone()),
            _ => Err(DecodeStringError),
        }
    }
}
#[derive(Debug)]
pub struct DecodeStringError;
impl fmt::Display for DecodeStringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "expected string")
    }
}
impl Error for DecodeStringError {}
impl EncodeCell for &str {
    type Error = Infallible;
    fn encode_cell(&self) -> Result<Value, Self::Error> {
        Ok(Value::String((*self).to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn string_encode_decode_roundtrip() {
        let s = "hello".to_string();
        // Encode
        let encoded = s.encode_cell().unwrap();
        assert_eq!(encoded, json!("hello"));

        // Decode
        let decoded = String::decode_cell(&json!("hello")).unwrap();
        assert_eq!(decoded, "hello");

        // Decode failure
        assert!(String::decode_cell(&json!(42)).is_err());
    }

    #[test]
    fn str_encode_roundtrip() {
        let s: &str = "world";
        let encoded = s.encode_cell().unwrap();
        assert_eq!(encoded, json!("world"));
    }
}
