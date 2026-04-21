use origin_asset::cognee::{CognifyOptions, SearchOptions, SearchType};
use origin_asset::OriginClient;

#[tokio::main]
async fn main() -> origin_asset::Result<()> {
    // Cognee uses a separate ca_xxx token
    let api_key = std::env::var("ORIGIN_API_KEY").expect("set ORIGIN_API_KEY env var");
    let cognee_token =
        std::env::var("COGNEE_TOKEN").unwrap_or_else(|_| api_key.clone());

    let client = OriginClient::builder(&api_key)
        .cognee_token(&cognee_token)
        .build();

    let cognee = client.cognee();

    // Health check
    let healthy = cognee.health().await?;
    println!("Cognee healthy: {healthy}");

    // List datasets
    let datasets = cognee.datasets().await?;
    println!("Datasets: {datasets}");

    // Add text to a dataset
    let result = cognee
        .add_text("my-knowledge", "Rust is a systems programming language.")
        .await?;
    println!("Added: {result}");

    // Trigger cognify
    let cognify_result = cognee
        .cognify(&CognifyOptions {
            datasets: Some(vec!["my-knowledge".into()]),
            run_in_background: Some(true),
            ..Default::default()
        })
        .await?;
    println!("Cognify: {cognify_result}");

    // Search knowledge
    let search_result = cognee
        .search(
            "What is Rust?",
            Some(SearchOptions {
                search_type: Some(SearchType::Summaries),
                top_k: Some(3),
                ..Default::default()
            }),
        )
        .await?;
    println!("Search result: {search_result}");

    Ok(())
}
