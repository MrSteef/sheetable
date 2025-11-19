use std::str::FromStr;

use a1_notation::A1;
use dotenv::dotenv;
use serde_json::Value;
use sheetable::providers::SpreadsheetProvider;
use sheetable::providers::google_sheets::GoogleSheetProvider;
use sheetable::{Sheetable, Table};
use sheetable::cell_encoding::{EncodeCell, DecodeCell};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let gsheet = GoogleSheetProvider::try_new_from_env().await?;

    let clear_range = A1::from_str("A1:B2").unwrap();
    gsheet.clear_range(&clear_range).await.unwrap();

    let first_range = A1::from_str("A1:B1").unwrap();
    gsheet
        .write_range(
            &first_range,
            vec![vec![Value::String("Hello world!".to_string())]],
        )
        .await
        .unwrap();

    let item = Item {
        first: "Hello".to_string(),
        second: "World".to_string(),
    };
    let second_range = A1::from_str("A2:B2").unwrap();
    gsheet
        .write_range(
            &second_range,
            vec![item.to_values()],
        )
        .await
        .unwrap();

    let table_range = A1::from_str("A:B").unwrap();
    let item_table: Table<Item, GoogleSheetProvider> = Table::new(&gsheet, table_range);

    let items = item_table.read_all().await?;

    println!("{:?}", items);

    // let returned_range = item_table.range_for_key(item.clone()).await.unwrap().unwrap();

    // println!("{item:?} is at {returned_range}");

    Ok(())
}

#[derive(Debug, Clone)]
pub struct Item {
    first: String,
    second: String,
}

impl Sheetable for Item {
    fn to_values(&self) -> Vec<Value> {
        vec![
            self.first.encode_cell().unwrap(),
            self.second.encode_cell().unwrap(),
        ]
    }

    fn from_values(values: Vec<Value>) -> Self {
        let first = values
            .get(0)
            .map(|v| String::decode_cell(v).unwrap_or_default())
            .unwrap_or_default();

        let second = values
            .get(1)
            .map(|v| String::decode_cell(v).unwrap_or_default())
            .unwrap_or_default();

        Item { first, second }
    }

    fn get_key(&self) -> Vec<Value> {
        vec![Value::String(self.first.clone())]
    }
}
