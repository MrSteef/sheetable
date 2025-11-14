use dotenv::dotenv;
use sheetable::GSheet;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let gsheet = GSheet::try_new().await?;
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

    let read = gsheet.read_range("A1:B2".to_string()).await.unwrap();
    let items = read.into_iter().map(Item::from_values).collect::<Vec<Item>>();
    println!("Read cells A1:B2 : {:?}", items);

    Ok(())
}

#[derive(Debug)]
pub struct Item {
    first: String,
    second: String,
}

impl Item {
    pub fn to_values(&self) -> Vec<serde_json::Value> {
        let first_value = serde_json::Value::String(self.first.clone());
        let second_value = serde_json::Value::String(self.second.clone());
        vec![first_value, second_value]
    }

    pub fn from_values(values: Vec<serde_json::Value>) -> Self {
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
}
