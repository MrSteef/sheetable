mod string;
use std::error::Error;

use serde_json::Value;

pub use string::*;
mod bool;
pub use bool::*;
mod numeric;
pub use numeric::*;
mod option;
#[allow(unused_imports)]
pub use option::*;

/// Convert a Rust value to a single `serde_json::Value` cell.
///
/// Implementors choose the [`Error`](EncodeCell::Error) type. Use
/// [`std::convert::Infallible`] when the conversion cannot fail.
pub trait EncodeCell {
    type Error: Error + Send + Sync + 'static;
    fn encode_cell(&self) -> Result<Value, Self::Error>;
}

/// Convert a single `serde_json::Value` cell to a Rust value.
///
/// Implementors choose the [`Error`](DecodeCell::Error) type (e.g. `ParseIntError`).
pub trait DecodeCell: Sized {
    type Error: Error + Send + Sync + 'static;
    fn decode_cell(value: &Value) -> Result<Self, Self::Error>;
}
