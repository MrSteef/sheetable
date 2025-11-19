use a1_notation::A1;
use serde_json::Value;
use std::error::Error as StdError;
use std::fmt::Debug;

pub mod google_sheets;

/// Result of reading a range from a provider.
#[derive(Debug, Clone)]
pub struct RangeResult {
    /// 2D values, row-major: Vec<row>[Vec<cell>]
    pub values: Vec<Vec<Value>>,

    /// Effective A1 range returned by the provider.
    pub range: Option<A1>,
}

#[allow(async_fn_in_trait)]
pub trait SpreadsheetProvider: Debug + Send + Sync {
    /// Provider-specific error type.
    type Error: StdError + Send + Sync + 'static;

    /// Read a 2D range of values.
    async fn read_range(&self, range: &A1) -> Result<RangeResult, Self::Error>;

    /// Write values in a range.
    async fn write_range(
        &self,
        range: &A1,
        values: Vec<Vec<Value>>,
    ) -> Result<(), Self::Error>;

    /// Append rows at (or below) a given range.
    /// `range` is the existing table range to append to.
    /// (e.g. "Sheet1!A:B" or "Sheet1!A2:B"), and the provider decides
    /// how to append rows under that anchor.
    async fn append_rows(
        &self,
        range: &A1,
        values: Vec<Vec<Value>>,
    ) -> Result<(), Self::Error>;

    /// Delete row(s) within a sheet by index.
    /// The column part of the provided A1 range is ignored.
    async fn delete_rows(
        &self,
        range: &A1,
    ) -> Result<(), Self::Error>;

    /// Clear a range without fully deleting any rows or columns.
    async fn clear_range(
        &self,
        range: &A1,
    ) -> Result<(), Self::Error>;
}
