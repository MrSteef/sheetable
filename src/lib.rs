use a1_notation::A1;
use serde_json::Value;
use std::error::Error as StdError;
use thiserror::Error;

use crate::providers::{RangeResult, SpreadsheetProvider};

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

    pub async fn range_for_key(&self, item: &T) -> Result<Option<A1>, P::Error> {
        let RangeResult { values, range } = self.provider.read_range(&self.range).await?;

        if values.is_empty() {
            return Ok(None);
        }

        let Some(index) = values
            .into_iter()
            .position(|row| T::from_values(row).get_key() == item.get_key())
        else {
            return Ok(None);
        };

        let first_row = match range.reference {
            a1_notation::RangeOrCell::Cell(address) => address.row.y,
            a1_notation::RangeOrCell::ColumnRange {..} => 0,
            a1_notation::RangeOrCell::NonContiguous(_) => todo!(),
            a1_notation::RangeOrCell::Range { from, .. } => from.row.y,
            a1_notation::RangeOrCell::RowRange { from, .. } => from.y,
        };
        let row_range = range.with_y(first_row + index);

        Ok(Some(row_range))
    }
    
    pub async fn create(&self, item: &T) -> Result<(), P::Error> {
        self.provider.append_rows(&self.range, vec![item.to_values()]).await
    }

    pub async fn edit(&self, item: &T) -> Result<(), P::Error> {
        let range = self.range_for_key(&item).await?.unwrap(); // TODO: handle this properly
        self.provider.write_range(&range, vec![item.to_values()]).await?;

        Ok(())
    }

    pub async fn delete(&self, item: T) -> Result<(), P::Error> {
        let range = self.range_for_key(&item).await?.unwrap(); // TODO: handle this properly
        self.provider.delete_rows(&range).await?;

        Ok(())
    }
}
