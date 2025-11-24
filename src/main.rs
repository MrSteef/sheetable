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

    let clear_range = A1::from_str("A:B").unwrap();
    gsheet.clear_range(&clear_range).await.unwrap();

    let table_range = A1::from_str("A:B").unwrap();
    let item_table: Table<Item, GoogleSheetProvider> = Table::new(&gsheet, table_range);

    let mut first_item = Item {
        first: "Hello".to_string(),
        second: "World".to_string(),
    };
    item_table.create(&first_item).await?;

    let second_item = Item {
        first: "Hi".to_string(),
        second: "there!".to_string(),
    };
    item_table.create(&second_item).await?;

    first_item.second = "World!".to_string();
    item_table.edit(&first_item).await?;

    let third_item = Item {
        first: "Goodbye".to_string(),
        second: "World".to_string(),
    };
    item_table.create(&third_item).await?;

    item_table.delete(second_item).await?;

    let items = item_table.read_all().await?;

    println!("{:?}", items);

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
