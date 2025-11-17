use dotenv::dotenv;
use sheetable::{GSheet, Sheetable, Table};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let gsheet = GSheet::try_new().await?;

    gsheet.clear_range("Blad1!A:B".to_string()).await?;

    gsheet
        .write_cell(
            "A1:B1".to_string(),
            serde_json::Value::String("Hello world!".to_string()),
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
    fn to_values(&self) -> Vec<serde_json::Value> {
        let first_value = serde_json::Value::String(self.first.clone());
        let second_value = serde_json::Value::String(self.second.clone());
        vec![first_value, second_value]
    }

    fn from_values(values: Vec<serde_json::Value>) -> Self {
        let first = values
            .get(0)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let second = values
            .get(1)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        Item { first, second }
    }

    fn get_key(&self) -> Vec<serde_json::Value> {
        vec![serde_json::Value::String(self.first.clone())]
    }
}
