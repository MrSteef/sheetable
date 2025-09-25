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

mod traits;
pub use traits::{Sheetable, SheetableReadOnly};

pub mod errors;
pub use errors::{Result, SheetError};

pub mod cell_encoding;
pub use cell_encoding::{DecodeCell, EncodeCell};
