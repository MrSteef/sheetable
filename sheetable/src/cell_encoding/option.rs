use serde_json::Value;

use crate::cell_encoding::{DecodeCell, EncodeCell};

impl<T: EncodeCell> EncodeCell for Option<T> {
    type Error = T::Error;
    fn encode_cell(&self) -> Result<Value, Self::Error> {
        match self {
            Some(t) => t.encode_cell(),
            None => Ok(Value::Null),
        }
    }
}
impl<T: DecodeCell> DecodeCell for Option<T> {
    type Error = T::Error;
    fn decode_cell(v: &Value) -> Result<Self, Self::Error> {
        if v.is_null() {
            Ok(None)
        } else {
            T::decode_cell(v).map(Some)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn option_roundtrip() {
        let some_val: Option<u64> = Some(42);
        let encoded = some_val.encode_cell().unwrap();
        assert_eq!(encoded, json!(42));
        let decoded = Option::<u64>::decode_cell(&json!(42)).unwrap();
        assert_eq!(decoded, Some(42));

        let none_val: Option<u64> = None;
        let encoded_none = none_val.encode_cell().unwrap();
        assert_eq!(encoded_none, json!(null));
        let decoded_none = Option::<u64>::decode_cell(&json!(null)).unwrap();
        assert_eq!(decoded_none, None);
    }
}
