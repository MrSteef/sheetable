use a1_notation::A1;
use google_sheets4::{
    Sheets,
    api::ValueRange,
    hyper_rustls::{self, HttpsConnector},
    hyper_util::{self, client::legacy::connect::HttpConnector},
    yup_oauth2::{ServiceAccountAuthenticator, ServiceAccountKey},
};
use serde_json::Value;
use std::{fmt, str::FromStr};
use std::{env, fs::File, io::Read, sync::Arc};
use tokio::sync::Mutex;

pub mod cell_encoding;

pub struct GSheet {
    pub sheets: Arc<Mutex<Sheets<HttpsConnector<HttpConnector>>>>,
    pub document_id: String,
}

impl fmt::Debug for GSheet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GSheet")
            .field("document_id", &self.document_id)
            .field("sheets", &"<omitted>")
            .finish()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GSheetError {
    #[error(transparent)]
    Env(#[from] std::env::VarError),

    #[error(transparent)]
    ServiceAccount(#[from] ServiceAccountError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl GSheet {
    pub async fn try_new() -> Result<Self, GSheetError> {
        let document_id = env::var("GOOGLE_SHEET_ID")?;
        let service_account_path = env::var("SERVICE_ACCOUNT_JSON")?;
        let service_account = read_service_account_json(&service_account_path)?;
        let builder = ServiceAccountAuthenticator::builder(service_account);
        let auth = builder.build().await?;
        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build(
                    hyper_rustls::HttpsConnectorBuilder::new()
                        .with_native_roots()
                        .unwrap()
                        .https_or_http()
                        .enable_http1()
                        .build(),
                );
        let sheets: Sheets<HttpsConnector<HttpConnector>> = Sheets::new(client, auth);

        sheets.spreadsheets();

        Ok(GSheet {
            sheets: Arc::new(Mutex::new(sheets)),
            document_id,
        })
    }

    pub async fn write_cell(
        &self,
        cell: String,
        value: Value,
    ) -> Result<(), google_sheets4::Error> {
        let values = vec![vec![value]];

        let request: ValueRange = ValueRange {
            major_dimension: Some("ROWS".to_owned()),
            range: Some(cell.clone()),
            values: Some(values),
        };

        let sheets = self.sheets.lock().await;

        sheets
            .spreadsheets()
            .values_update(request, &self.document_id, &cell)
            .value_input_option("RAW")
            .doit()
            .await?;

        Ok(())
    }

    pub async fn write_range(
        &self,
        range: String,
        values: Vec<Value>,
    ) -> Result<(), google_sheets4::Error> {
        let values = vec![values];

        let request: ValueRange = ValueRange {
            major_dimension: Some("ROWS".to_owned()),
            range: Some(range.clone()),
            values: Some(values),
        };

        let sheets = self.sheets.lock().await;

        sheets
            .spreadsheets()
            .values_update(request, &self.document_id, &range)
            .value_input_option("RAW")
            .doit()
            .await?;

        Ok(())
    }

    pub async fn read_cell(&self, cell: String) -> Result<Value, google_sheets4::Error> {
        let sheets = self.sheets.lock().await;

        let (_, result) = sheets
            .spreadsheets()
            .values_get(&self.document_id, &cell)
            .doit()
            .await?;
        let mut values = result.values.unwrap(); // make this an actual error
        let mut row = values.pop().unwrap(); // make this an actual error
        let cell = row.pop().unwrap(); // make this an actual error

        Ok(cell)
    }

    pub async fn read_range(&self, range: String) -> Result<Vec<Vec<Value>>, google_sheets4::Error> {
        let sheets = self.sheets.lock().await;

        let (_, result) = sheets
            .spreadsheets()
            .values_get(&self.document_id, &range)
            .doit()
            .await?;
        let values = result.values.unwrap(); // make this an actual error


        Ok(values)
    }

    pub async fn read_range_with_meta(&self, range: String) -> Result<(Vec<Vec<Value>>, String), google_sheets4::Error> {
        let sheets = self.sheets.lock().await;

        let (_, result) = sheets
            .spreadsheets()
            .values_get(&self.document_id, &range)
            .doit()
            .await?;
        let values = result.values.unwrap(); // make this an actual error
        let range_meta = result.range.unwrap();

        Ok((values, range_meta))
    }

    pub async fn clear_range(
        &self,
        range: String,
    ) -> Result<(), google_sheets4::Error> {
        let request = google_sheets4::api::ClearValuesRequest::default();

        let sheets = self.sheets.lock().await;

        sheets
            .spreadsheets()
            .values_clear(request, &self.document_id, &range)
            .doit()
            .await?;

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ServiceAccountError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

fn read_service_account_json(file_path: &str) -> Result<ServiceAccountKey, ServiceAccountError> {
    let mut file = File::open(file_path)?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let acc: ServiceAccountKey = serde_json::from_str(&contents)?;

    Ok(acc)
}

pub trait Sheetable {
    fn to_values(&self) -> Vec<Value>;
    fn from_values(values: Vec<Value>) -> Self;
    fn get_key(&self) -> Vec<Value>;
}

pub struct Table<'a, T: Sheetable> {
    gsheet: &'a GSheet,
    range: String,
    _marker: std::marker::PhantomData<T>,
}

impl<'a, T: Sheetable> Table<'a, T> {
    pub fn new(gsheet: &'a GSheet,
    range: impl Into<String>) -> Self {
        Table {
            gsheet,
            range: range.into(),
            _marker: std::marker::PhantomData,
        }
    }

    pub async fn read_all(&self) -> Result<Vec<T>, google_sheets4::Error> {
        let rows = self.gsheet.read_range(self.range.clone()).await?;
        let items = rows.into_iter().map(T::from_values).collect();
        Ok(items)
    }

    pub async fn range_for_key(
        &self,
        item: T,
    ) -> Result<Option<String>, google_sheets4::Error> {
        let (values, range_meta) = self.gsheet.read_range_with_meta(self.range.clone()).await?;

        if values.is_empty() {
            return Ok(None);
        }

        let index = values.into_iter().position(|row| T::from_values(row).get_key() == item.get_key()).unwrap_or(0);

        let full_range = A1::from_str(&range_meta).unwrap();

        let first_row = match full_range.reference {
            a1_notation::RangeOrCell::Cell(address) => address.row.y,
            a1_notation::RangeOrCell::ColumnRange {..} => 0,
            a1_notation::RangeOrCell::NonContiguous(_) => todo!(),
            a1_notation::RangeOrCell::Range { from, .. } => from.row.y,
            a1_notation::RangeOrCell::RowRange { from, .. } => from.y,
        };
        let row_range = full_range.with_y(first_row + index).to_string();

        Ok(Some(row_range))
    }
}

impl GSheet {
    pub fn table<'a, T>(&'a self, range: impl Into<String>) -> Table<'a, T>
    where
        T: Sheetable,
    {
        Table::new(self, range)
    }
}