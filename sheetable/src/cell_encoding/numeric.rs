use std::{convert::Infallible, error::Error, fmt};

use serde_json::{Number, Value};

use crate::cell_encoding::{DecodeCell, EncodeCell};

impl EncodeCell for u64 {
    type Error = Infallible;
    fn encode_cell(&self) -> Result<Value, Self::Error> {
        Ok(Value::Number(Number::from(*self)))
    }
}
impl DecodeCell for u64 {
    type Error = std::num::ParseIntError;
    fn decode_cell(v: &Value) -> Result<Self, Self::Error> {
        match v {
            Value::Number(n) => n.as_u64().ok_or_else(|| "0".parse::<u64>().unwrap_err()),
            Value::String(s) => s.parse::<u64>(),
            _ => "x".parse::<u64>(),
        }
    }
}

impl EncodeCell for i64 {
    type Error = Infallible;
    fn encode_cell(&self) -> Result<Value, Self::Error> {
        Ok(Value::Number(Number::from(*self)))
    }
}
impl DecodeCell for i64 {
    type Error = std::num::ParseIntError;
    fn decode_cell(v: &Value) -> Result<Self, Self::Error> {
        match v {
            Value::Number(n) => n.as_i64().ok_or_else(|| "x".parse::<i64>().unwrap_err()),
            Value::String(s) => s.parse::<i64>(),
            _ => "x".parse::<i64>(),
        }
    }
}

impl EncodeCell for f64 {
    type Error = EncodeFloatError;
    fn encode_cell(&self) -> Result<Value, Self::Error> {
        Number::from_f64(*self)
            .map(Value::Number)
            .ok_or(EncodeFloatError)
    }
}
impl DecodeCell for f64 {
    type Error = std::num::ParseFloatError;
    fn decode_cell(v: &Value) -> Result<Self, Self::Error> {
        match v {
            Value::Number(n) => n.as_f64().ok_or_else(|| "x".parse::<f64>().unwrap_err()),
            Value::String(s) => s.parse::<f64>(),
            _ => "x".parse::<f64>(),
        }
    }
}
#[derive(Debug)]
pub struct EncodeFloatError;
impl fmt::Display for EncodeFloatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid f64 (NaN/Inf)")
    }
}
impl Error for EncodeFloatError {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn u64_roundtrip() {
        let n: u64 = 42;
        let encoded = n.encode_cell().unwrap();
        assert_eq!(encoded, json!(42));
        let decoded = u64::decode_cell(&json!(42)).unwrap();
        assert_eq!(decoded, 42);
        let decoded_str = u64::decode_cell(&json!("42")).unwrap();
        assert_eq!(decoded_str, 42);
    }

    #[test]
    fn i64_roundtrip() {
        let n: i64 = -7;
        let encoded = n.encode_cell().unwrap();
        assert_eq!(encoded, json!(-7));
        let decoded = i64::decode_cell(&json!(-7)).unwrap();
        assert_eq!(decoded, -7);
        let decoded_str = i64::decode_cell(&json!("-7")).unwrap();
        assert_eq!(decoded_str, -7);
    }

    #[test]
    fn f64_roundtrip() {
        let x: f64 = 3.25;
        let encoded = x.encode_cell().unwrap();
        assert_eq!(encoded, json!(3.25));
        let decoded = f64::decode_cell(&json!(3.25)).unwrap();
        assert_eq!(decoded, 3.25);

        // Encode NaN/Inf should error
        let nan = f64::NAN;
        assert!(f64::encode_cell(&nan).is_err());
    }
}
