use dotenv::dotenv;
use serde_json::Value;
use sheetable::{GSheet, Sheetable, Table};
use sheetable::cell_encoding::{EncodeCell, DecodeCell};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let gsheet = GSheet::try_new().await?;

    gsheet.clear_range("Blad1!A:B".to_string()).await?;

    gsheet
        .write_cell(
            "A1:B1".to_string(),
            Value::String("Hello world!".to_string()),
        )
        .await
        .unwrap();

    let item = Item {
        first: "Hello".to_string(),
        second: "World".to_string(),
    };
    gsheet
        .write_range(
            "A2:B2".to_string(),
            item.to_values(),
        )
        .await
        .unwrap();

    let item_table: Table<Item> = gsheet.table("A:B");

    let items = item_table.read_all().await?;

    println!("{:?}", items);

    let returned_range = item_table.range_for_key(item.clone()).await.unwrap().unwrap();

    println!("{item:?} is at {returned_range}");



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
