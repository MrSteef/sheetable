use dotenv::dotenv;
use sheetable::GSheet;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let gsheet = GSheet::try_new().await?;
    gsheet.write_cell("A1".to_string(), serde_json::Value::String("Hello world!".to_string())).await.unwrap();

    Ok(())
}