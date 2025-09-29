//! Spreadsheet client abstraction for Sheetable rows.
//!
//! This trait allows Sheetable structs to be read and written
//! from/to any spreadsheet provider (Google Sheets, Excel Online, etc.).
//!
//! # Overview
//!
//! `SpreadsheetClient` is provider-agnostic. It exposes basic operations:
//! - Getting a table range
//! - Appending a row
//! - Updating a cell
//!
//! Implementors can choose how authentication is handled internally.

use std::error::Error;
use std::fmt;

/// A generic error type for spreadsheet operations.
#[derive(Debug)]
pub enum SpreadsheetError {
    /// The requested sheet or range was not found.
    NotFound(String),
    /// An error from the underlying provider (API/network).
    Provider(String),
    /// Generic message.
    Message(String),
}

impl fmt::Display for SpreadsheetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpreadsheetError::NotFound(msg) => write!(f, "Sheet or range not found: {}", msg),
            SpreadsheetError::Provider(msg) => write!(f, "Provider error: {}", msg),
            SpreadsheetError::Message(msg) => write!(f, "{}", msg),
        }
    }
}

impl Error for SpreadsheetError {}

/// Trait representing a generic spreadsheet client.
///
/// Providers like Google Sheets or Excel Online can implement this
/// trait. All operations use **sheet names** and **column ranges**
/// derived from `Sheetable` structs.
pub trait SpreadsheetClient {
    /// Fetch a rectangular range of cells for the given sheet and columns.
    ///
    /// # Arguments
    /// * `sheet_name` - The name of the sheet/tab.
    /// * `columns` - Column letters to read (e.g., `["A","B","C"]`).
    ///
    /// Returns a 2D vector representing rows x columns.
    fn get_table_range(
        &self,
        sheet_name: &str,
        columns: &[&str],
    ) -> Result<Vec<Vec<String>>, SpreadsheetError>;

    /// Append a single row of values to a sheet.
    fn append_row(&self, sheet_name: &str, row: &[String]) -> Result<(), SpreadsheetError>;

    /// Update a single cell.
    ///
    /// # Arguments
    /// * `sheet_name` - The name of the sheet/tab.
    /// * `range` - The cell range (e.g., `"B2:B2"`).
    /// * `value` - The new value.
    fn update_cell(
        &self,
        sheet_name: &str,
        range: &str,
        value: &str,
    ) -> Result<(), SpreadsheetError>;
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;

    use super::*;

    /// Dummy client using a simple 2D table per sheet.
    struct DummyClient {
        /// Map of sheet name → table (rows × columns)
        tables: RefCell<HashMap<String, Vec<Vec<String>>>>,
    }

    impl DummyClient {
        fn new() -> Self {
            Self {
                tables: RefCell::new(HashMap::new()),
            }
        }
    }

    impl SpreadsheetClient for DummyClient {
        fn get_table_range(
            &self,
            sheet_name: &str,
            _columns: &[&str],
        ) -> Result<Vec<Vec<String>>, SpreadsheetError> {
            Ok(self
                .tables
                .borrow()
                .get(sheet_name)
                .cloned()
                .unwrap_or_default())
        }

        fn append_row(&self, sheet_name: &str, row: &[String]) -> Result<(), SpreadsheetError> {
            let mut tables = self.tables.borrow_mut();
            let table = tables.entry(sheet_name.to_string()).or_default();
            table.push(row.to_vec());
            Ok(())
        }

        fn update_cell(
            &self,
            sheet_name: &str,
            range: &str,
            value: &str,
        ) -> Result<(), SpreadsheetError> {
            let mut tables = self.tables.borrow_mut();
            let table = tables
                .get_mut(sheet_name)
                .ok_or_else(|| SpreadsheetError::NotFound(sheet_name.to_string()))?;
            let col_index = range.chars().next().unwrap() as usize - 'A' as usize;
            let row_index: usize = range[1..].parse::<usize>().unwrap() - 1; // specify type here

            if row_index >= table.len() || col_index >= table[row_index].len() {
                return Err(SpreadsheetError::Message(format!(
                    "Cell {} is out of bounds in sheet '{}'",
                    range, sheet_name
                )));
            }

            table[row_index][col_index] = value.to_string();
            Ok(())
        }
    }

    #[test]
    fn append_and_get_table_range() {
        let client = DummyClient::new();
        client
            .append_row("Sheet1", &vec!["A1".to_string(), "B1".to_string()])
            .unwrap();
        client
            .append_row("Sheet1", &vec!["A2".to_string(), "B2".to_string()])
            .unwrap();

        let data = client.get_table_range("Sheet1", &["A", "B"]).unwrap();
        assert_eq!(data.len(), 2);
        assert_eq!(data[0], vec!["A1", "B1"]);
        assert_eq!(data[1], vec!["A2", "B2"]);
    }

    #[test]
    fn update_cell_works() {
        let client = DummyClient::new();
        client
            .append_row("Sheet1", &vec!["old".to_string(), "B".to_string()])
            .unwrap();

        client.update_cell("Sheet1", "A1", "new").unwrap();
        let data = client.get_table_range("Sheet1", &["A", "B"]).unwrap();
        assert_eq!(data[0][0], "new");
        assert_eq!(data[0][1], "B");

        // Updating an out-of-bounds cell should return an error
        let err = client.update_cell("Sheet1", "C1", "x").unwrap_err();
        match err {
            SpreadsheetError::Message(_) => {}
            _ => panic!("Expected Message error"),
        }
    }

    #[test]
    fn get_table_from_empty_sheet_returns_empty() {
        let client = DummyClient::new();
        let data = client.get_table_range("SheetX", &["A", "B"]).unwrap();
        assert!(data.is_empty());
    }
}
