use a1_notation::A1;
use google_sheets4::api::{
    BatchUpdateSpreadsheetRequest, DeleteDimensionRequest, DimensionRange, Request, ValueRange,
    ClearValuesRequest,
};
use google_sheets4::yup_oauth2::{ServiceAccountAuthenticator, ServiceAccountKey};
use google_sheets4::{
    hyper, hyper_rustls,
    hyper_rustls::HttpsConnector,
    hyper_util::{self, client::legacy::connect::HttpConnector},
    Sheets,
};
use serde_json::{Error as JsonError, Value};
use std::{env, fmt, fs, io, str::FromStr, sync::Arc};
use thiserror::Error;
use tokio::sync::Mutex;

use crate::providers::{RangeResult, SpreadsheetProvider};

pub struct GoogleSheetProvider {
    sheets: Arc<Mutex<Sheets<HttpsConnector<HttpConnector>>>>,
    pub document_id: String,
}

impl fmt::Debug for GoogleSheetProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GoogleSheetProvider")
            .field("document_id", &self.document_id)
            .finish()
    }
}

impl GoogleSheetProvider {
    pub async fn try_new_from_env() -> Result<Self, GoogleSheetError> {
        let document_id = env::var("GOOGLE_SHEET_ID")?;
        let service_account_path = env::var("SERVICE_ACCOUNT_JSON")?;

        let service_account = read_service_account_json(&service_account_path)?;

        let auth = ServiceAccountAuthenticator::builder(service_account)
            .build()
            .await
            .map_err(|e| GoogleSheetError::Auth(e.to_string()))?;

        let https = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .map_err(|e| GoogleSheetError::TlsConfig(e.to_string()))?
            .https_or_http()
            .enable_http1()
            .build();

        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build(https);

        let sheets = Sheets::new(client, auth);

        Ok(GoogleSheetProvider {
            sheets: Arc::new(Mutex::new(sheets)),
            document_id,
        })
    }
}

fn read_service_account_json(file_path: &str) -> Result<ServiceAccountKey, GoogleSheetError> {
    let contents = fs::read_to_string(file_path)?;
    let acc = serde_json::from_str(&contents)?;
    Ok(acc)
}

impl SpreadsheetProvider for GoogleSheetProvider {
    type Error = GoogleSheetError;

    async fn read_range(&self, range: &A1) -> Result<RangeResult, Self::Error> {
        let range_str = range.to_string();
        let sheets = self.sheets.lock().await;

        let (_, result) = sheets
            .spreadsheets()
            .values_get(&self.document_id, &range_str)
            .doit()
            .await?;

        let values = result
            .values
            .ok_or_else(|| GoogleSheetError::EmptyRange(range_str.clone()))?;

        let returned_range = result
            .range
            .as_deref()
            .ok_or_else(|| GoogleSheetError::MissingEffectiveRange(range_str.clone()))
            .and_then(|rs| {
                A1::from_str(rs).map_err(|e| GoogleSheetError::InvalidA1Range {
                    range: rs.to_owned(),
                    reason: e.to_string(),
                })
            })?;

        Ok(RangeResult {
            values,
            range: Some(returned_range),
        })
    }

    async fn write_range(
        &self,
        range: &A1,
        values: Vec<Vec<Value>>,
    ) -> Result<(), Self::Error> {
        let range_str = range.to_string();

        let request = ValueRange {
            major_dimension: Some("ROWS".to_owned()),
            range: Some(range_str.clone()),
            values: Some(values),
        };

        let sheets = self.sheets.lock().await;

        sheets
            .spreadsheets()
            .values_update(request, &self.document_id, &range_str)
            .value_input_option("RAW")
            .doit()
            .await?;

        Ok(())
    }

    async fn append_rows(
        &self,
        range: &A1,
        values: Vec<Vec<Value>>,
    ) -> Result<(), Self::Error> {
        let range_str = range.to_string();

        let request = ValueRange {
            major_dimension: Some("ROWS".to_owned()),
            range: Some(range_str.clone()),
            values: Some(values),
        };

        let sheets = self.sheets.lock().await;

        sheets
            .spreadsheets()
            .values_append(request, &self.document_id, &range_str)
            .value_input_option("RAW")
            .doit()
            .await?;

        Ok(())
    }

    async fn delete_rows(&self, range: &A1) -> Result<(), Self::Error> {
        use a1_notation::RangeOrCell;

        let (start_index, end_index) = match &range.reference {
            RangeOrCell::Cell(address) => (address.row.y, address.row.y),
            RangeOrCell::Range { from, to } => (from.row.y, to.row.y),
            RangeOrCell::RowRange { from, to } => (from.y, to.y),

            RangeOrCell::ColumnRange { .. } => {
                return Err(GoogleSheetError::InvalidA1Range {
                    range: range.reference.to_string(),
                    reason: "Range for deleting rows may not be a column range".to_owned(),
                });
            }
            RangeOrCell::NonContiguous(..) => {
                return Err(GoogleSheetError::InvalidA1Range {
                    range: range.reference.to_string(),
                    reason: "Range for deleting rows may not be non contiguous".to_owned(),
                });
            }
        };

        let sheet_name = range
            .sheet_name
            .clone()
            .ok_or_else(|| GoogleSheetError::InvalidA1Range {
                range: range.to_string(),
                reason: "Range must include a sheet name".to_owned(),
            })?;

        let sheets_lock = self.sheets.lock().await;

        let (_, spreadsheet) = sheets_lock
            .spreadsheets()
            .get(&self.document_id)
            .doit()
            .await?;

        let sheet_id = spreadsheet
            .sheets
            .as_deref()
            .ok_or_else(|| {
                GoogleSheetError::MissingEffectiveRange(
                    "No sheets were found in the document".to_owned(),
                )
            })?
            .iter()
            .find_map(|s| {
                let props = s.properties.as_ref()?;
                (props.title.as_deref() == Some(&sheet_name)).then_some(props.sheet_id)
            })
            .flatten()
            .ok_or_else(|| {
                GoogleSheetError::MissingEffectiveRange(format!(
                    "Unable to obtain sheet id for sheet '{sheet_name}'",
                ))
            })?;

        let request = BatchUpdateSpreadsheetRequest {
            requests: Some(vec![Request {
                delete_dimension: Some(DeleteDimensionRequest {
                    range: Some(DimensionRange {
                        sheet_id: Some(sheet_id),
                        dimension: Some("ROWS".to_owned()),
                        start_index: Some(start_index as i32),
                        end_index: Some(end_index as i32),
                    }),
                }),
                ..Default::default()
            }]),
            ..Default::default()
        };

        sheets_lock
            .spreadsheets()
            .batch_update(request, &self.document_id)
            .doit()
            .await?;

        Ok(())
    }

    async fn clear_range(&self, range: &A1) -> Result<(), Self::Error> {
        let range_str = range.to_string();
        let sheets = self.sheets.lock().await;

        sheets
            .spreadsheets()
            .values_clear(ClearValuesRequest::default(), &self.document_id, &range_str)
            .doit()
            .await?;

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum GoogleSheetError {
    #[error("environment variable error: {0}")]
    EnvVar(#[from] std::env::VarError),

    #[error("failed to read service account JSON: {0}")]
    ServiceAccountIo(#[from] io::Error),

    #[error("invalid service account JSON: {0}")]
    ServiceAccountJson(#[from] JsonError),

    #[error("OAuth authentication failed: {0}")]
    Auth(String),

    #[error("TLS configuration error: {0}")]
    TlsConfig(String),

    #[error("HTTP transport error: {0}")]
    HttpTransport(#[from] hyper::Error),

    #[error("Google Sheets API error: {0}")]
    Api(#[from] google_sheets4::Error),

    #[error("no values returned for range '{0}'")]
    EmptyRange(String),

    #[error("provider returned no effective range metadata for '{0}'")]
    MissingEffectiveRange(String),

    #[error("Invalid A1 range '{range}': {reason}")]
    InvalidA1Range { range: String, reason: String },
}
