//! # sheetable
//!
//! Core traits for mapping Rust structs to spreadsheet rows using
//! `serde_json::Value` as the cell representation.
//!
//! The companion `sheetable-derive` crate (when used) can generate implementations
//! automatically from struct attributes like `#[column("A")]` for writable fields
//! and `#[calculated(DetailsType)]` for fields that are calculated in the sheet.
//!
//! ## Quick start
//!
//! - Implement or derive [`Sheetable`] for your **row** struct.
//! - Implement or derive [`SheetableReadOnly`] for your **calculated** bundle.
//! - Implement or rely on provided impls of [`EncodeCell`] and [`DecodeCell`] for
//!   the field types you use.
//!
//! ### Example (conceptual, using the derive crate)
//! ```ignore
//! use sheetable::{Sheetable, SheetableReadOnly};
//! use serde_json::Value;
//!
//! #[derive(Sheetable)]
//! struct User<RO> {
//!     #[column("A")]
//!     id: u64,
//!     #[column("B")]
//!     name: String,
//!     #[calculated(UserDetails)]
//!     details: RO,
//! }
//!
//! #[derive(SheetableReadOnly)]
//! struct UserDetails {
//!     #[column("C")]
//!     elo: u64,
//! }
//!
//! // Writing (encodes only writable columns):
//! // user_instance.to_values()? -> Vec<Value>
//!
//! // Reading (returns hydrated instance):
//! // let hydrated: User<UserDetails> = User::<()>::from_values(&cells)?;
//! ```

use serde_json::{Number, Value};
use std::{convert::Infallible, error::Error as StdError, fmt};

/* -------------------------------------------------------------------------- */
/*                               Cell conversion                              */
/* -------------------------------------------------------------------------- */

/// Convert a Rust value to a single `serde_json::Value` cell.
///
/// Implementors choose the [`Error`](EncodeCell::Error) type. Use
/// [`Infallible`] when the conversion cannot fail.
pub trait EncodeCell {
    type Error: StdError + Send + Sync + 'static;
    fn encode_cell(&self) -> Result<Value, Self::Error>;
}

/// Convert a single `serde_json::Value` cell to a Rust value.
///
/// Implementors choose the [`Error`](DecodeCell::Error) type (e.g. `ParseIntError`).
pub trait DecodeCell: Sized {
    type Error: StdError + Send + Sync + 'static;
    fn decode_cell(value: &Value) -> Result<Self, Self::Error>;
}

/* -------------------------------------------------------------------------- */
/*                                Row-level API                               */
/* -------------------------------------------------------------------------- */

/// Struct-level error type used by [`Sheetable`] and [`SheetableReadOnly`].
///
/// Per-field conversion errors are wrapped in this type to keep public APIs
/// ergonomic and consistent.
#[derive(Debug)]
pub enum SheetError {
    /// A required value was missing at the provided index in a row.
    MissingValue { index: usize },

    /// A field failed to encode.
    Encode {
        field: &'static str,
        source: Box<dyn StdError + Send + Sync>,
    },

    /// A field failed to decode.
    Decode {
        field: &'static str,
        source: Box<dyn StdError + Send + Sync>,
    },

    /// Generic message.
    Message(String),
}

impl SheetError {
    /// Construct a `MissingValue` error.
    pub fn missing(index: usize) -> Self {
        SheetError::MissingValue { index }
    }

    /// Wrap an encode error for the named field.
    pub fn encode<E>(field: &'static str, err: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        SheetError::Encode {
            field,
            source: Box::new(err),
        }
    }

    /// Wrap a decode error for the named field.
    pub fn decode<E>(field: &'static str, err: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        SheetError::Decode {
            field,
            source: Box::new(err),
        }
    }
}

impl fmt::Display for SheetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SheetError::MissingValue { index } => write!(f, "missing value at index {index}"),
            SheetError::Encode { field, source } => write!(f, "encode error in `{field}`: {source}"),
            SheetError::Decode { field, source } => write!(f, "decode error in `{field}`: {source}"),
            SheetError::Message(s) => write!(f, "{s}"),
        }
    }
}
impl StdError for SheetError {}

/// Bundle of read-only (calculated) columns decoded from the sheet.
///
/// Types that represent calculated columns implement this trait. They are never
/// written back to the sheet.
pub trait SheetableReadOnly: Sized {
    /// Decode the read-only bundle from a slice of cells.
    fn from_values(values: &[Value]) -> Result<Self, SheetError>;
}

/// Mapping for a row struct with writable columns.
///
/// Implementations should encode only writable columns in [`to_values`], and
/// decode both writable and read-only columns in [`from_values`]. The return
/// type of [`from_values`] is the concrete **hydrated** struct for the row:
/// - If a row has no calculated fields, `Hydrated` is typically `Self`.
/// - If a row has calculated fields, `Hydrated` is usually the same struct with
///   its read-only generic filled in (e.g. `User<UserDetails>`).
pub trait Sheetable: Sized {
    /// The read-only bundle type (use `()` if none).
    type ReadOnly: SheetableReadOnly;

    /// The concrete hydrated type produced by [`from_values`].
    type Hydrated;

    /// Encode writable columns to cells. Calculated fields are ignored.
    fn to_values(&self) -> Result<Vec<Value>, SheetError>;

    /// Decode a fully hydrated row from cells.
    fn from_values(values: &[Value]) -> Result<Self::Hydrated, SheetError>;
}

/* -------------------------------------------------------------------------- */
/*                           Default read-only implementation                  */
/* -------------------------------------------------------------------------- */

impl SheetableReadOnly for () {
    fn from_values(_: &[Value]) -> Result<Self, SheetError> {
        Ok(())
    }
}

/* -------------------------------------------------------------------------- */
/*                           Standard type implementations                     */
/* -------------------------------------------------------------------------- */

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
impl StdError for DecodeStringError {}

impl EncodeCell for &str {
    type Error = Infallible;
    fn encode_cell(&self) -> Result<Value, Self::Error> {
        Ok(Value::String((*self).to_owned()))
    }
}

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
        write!(
            f,
            "expected bool or \"true\"/\"false\"/\"1\"/\"0\" string"
        )
    }
}
impl StdError for DecodeBoolError {}

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
        Number::from_f64(*self).map(Value::Number).ok_or(EncodeFloatError)
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
impl StdError for EncodeFloatError {}

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

/* -------------------------------------------------------------------------- */
/*                                   Tests                                    */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn std_impls_roundtrip() {
        let s = "hi".to_string();
        assert_eq!(s.encode_cell().unwrap(), json!("hi"));
        assert_eq!(String::decode_cell(&json!("hi")).unwrap(), "hi");

        assert_eq!(true.encode_cell().unwrap(), json!(true));
        assert!(bool::decode_cell(&json!("1")).unwrap());
        assert!(!bool::decode_cell(&json!("0")).unwrap());

        let n: u64 = 42;
        assert_eq!(n.encode_cell().unwrap(), json!(42));
        assert_eq!(u64::decode_cell(&json!("42")).unwrap(), 42);

        let i: i64 = -7;
        assert_eq!(i.encode_cell().unwrap(), json!(-7));
        assert_eq!(i64::decode_cell(&json!("-7")).unwrap(), -7);

        let x = 3.25_f64;
        assert_eq!(x.encode_cell().unwrap(), json!(3.25));
        assert_eq!(f64::decode_cell(&json!(3.25)).unwrap(), 3.25);
        assert!(f64::encode_cell(&f64::NAN).is_err());
    }

    /// No calculated fields: hydrated == self.
    #[test]
    fn player_no_details_roundtrip() {
        #[derive(Debug, Clone)]
        struct Player {
            id: u64,
            name: String,
        }

        impl Sheetable for Player {
            type ReadOnly = ();
            type Hydrated = Player;

            fn to_values(&self) -> Result<Vec<Value>, SheetError> {
                Ok(vec![
                    self.id.encode_cell().map_err(|e| SheetError::encode("id", e))?,
                    self.name.encode_cell().map_err(|e| SheetError::encode("name", e))?,
                ])
            }

            fn from_values(values: &[Value]) -> Result<Self::Hydrated, SheetError> {
                if values.len() < 2 {
                    return Err(SheetError::missing(1));
                }
                Ok(Player {
                    id: u64::decode_cell(&values[0]).map_err(|e| SheetError::decode("id", e))?,
                    name: String::decode_cell(&values[1]).map_err(|e| SheetError::decode("name", e))?,
                })
            }
        }

        let p = Player { id: 7, name: "Rosalina".into() };
        let row = p.to_values().unwrap();
        assert_eq!(row, vec![json!(7), json!("Rosalina")]);

        let hydrated: Player = Player::from_values(&row).unwrap();
        assert_eq!(hydrated.name, "Rosalina");
    }

    /// With calculated fields: from_values returns a hydrated generic instance.
    #[test]
    fn user_with_details_hydrated_from_values() {
        #[derive(Debug, Clone)]
        struct User<RO = ()> {
            id: u64,
            name: String,
            details: RO, // calculated bundle in practice
        }

        #[derive(Debug, Clone)]
        struct UserDetails {
            elo: u64,
        }

        impl SheetableReadOnly for UserDetails {
            fn from_values(values: &[Value]) -> Result<Self, SheetError> {
                if values.len() < 3 {
                    return Err(SheetError::missing(2));
                }
                Ok(UserDetails {
                    elo: u64::decode_cell(&values[2]).map_err(|e| SheetError::decode("elo", e))?,
                })
            }
        }

        impl<RO> Sheetable for User<RO> {
            type ReadOnly = UserDetails;
            type Hydrated = User<UserDetails>;

            fn to_values(&self) -> Result<Vec<Value>, SheetError> {
                Ok(vec![
                    self.id.encode_cell().map_err(|e| SheetError::encode("id", e))?,
                    self.name.encode_cell().map_err(|e| SheetError::encode("name", e))?,
                ])
            }

            fn from_values(values: &[Value]) -> Result<Self::Hydrated, SheetError> {
                if values.len() < 3 {
                    return Err(SheetError::missing(2));
                }
                Ok(User::<UserDetails> {
                    id: u64::decode_cell(&values[0]).map_err(|e| SheetError::decode("id", e))?,
                    name: String::decode_cell(&values[1]).map_err(|e| SheetError::decode("name", e))?,
                    details: UserDetails::from_values(values)?,
                })
            }
        }

        let base = User { id: 1, name: "Mario".into(), details: () };
        assert_eq!(base.to_values().unwrap(), vec![json!(1), json!("Mario")]);

        let hydrated: User<UserDetails> =
            <User<()> as Sheetable>::from_values(&[json!(1), json!("Mario"), json!(1500)]).unwrap();
        assert_eq!(hydrated.details.elo, 1500);

        assert_eq!(hydrated.to_values().unwrap(), vec![json!(1), json!("Mario")]);
    }
}
