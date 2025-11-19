use a1_notation::A1;
use serde_json::Value;
use std::error::Error as StdError;
use thiserror::Error;

use crate::providers::SpreadsheetProvider;

pub mod cell_encoding;
pub mod providers;

pub trait Sheetable {
    fn to_values(&self) -> Vec<Value>;
    fn from_values(values: Vec<Value>) -> Self;
    fn get_key(&self) -> Vec<Value>;
}

#[derive(Debug, Error)]
pub enum SheetError {
    #[error("provider error: {0}")]
    Provider(#[source] Box<dyn StdError + Send + Sync>),

    #[error("invalid A1 range '{range}': {reason}")]
    InvalidA1Range { range: String, reason: String },

    #[error("empty range: {0}")]
    EmptyRange(String),
    // ...other high-level errors...
}

pub struct Table<'a, T, P>
where
    T: Sheetable,
    P: SpreadsheetProvider,
{
    provider: &'a P,
    range: A1,
    _marker: std::marker::PhantomData<T>,
}

impl<'a, T, P> Table<'a, T, P>
where
    T: Sheetable,
    P: SpreadsheetProvider,
{
    pub fn new(provider: &'a P, range: A1) -> Self {
        Table {
            provider,
            range: range,
            _marker: std::marker::PhantomData,
        }
    }

    pub async fn read_all(&self) -> Result<Vec<T>, P::Error> {
        let result = self.provider.read_range(&self.range).await?;
        let rows = result.values;
        let items = rows.into_iter().map(T::from_values).collect();
        Ok(items)
    }

    // pub async fn range_for_key(
    //     &self,
    //     item: T,
    // ) -> Result<Option<String>, google_sheets4::Error> {
    //     let (values, range_meta) = self.gsheet.read_range_with_meta(self.range.clone()).await?;

    //     if values.is_empty() {
    //         return Ok(None);
    //     }

    //     let index = values.into_iter().position(|row| T::from_values(row).get_key() == item.get_key()).unwrap_or(0);

    //     let full_range = A1::from_str(&range_meta).unwrap();

    //     let first_row = match full_range.reference {
    //         a1_notation::RangeOrCell::Cell(address) => address.row.y,
    //         a1_notation::RangeOrCell::ColumnRange {..} => 0,
    //         a1_notation::RangeOrCell::NonContiguous(_) => todo!(),
    //         a1_notation::RangeOrCell::Range { from, .. } => from.row.y,
    //         a1_notation::RangeOrCell::RowRange { from, .. } => from.y,
    //     };
    //     let row_range = full_range.with_y(first_row + index).to_string();

    //     Ok(Some(row_range))
    // }
}