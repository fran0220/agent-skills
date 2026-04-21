use origin_search::SearchClient;

#[tokio::main]
async fn main() -> origin_search::Result<()> {
    let client = SearchClient::new("your-api-key");

    let fast = client.search_fast("latest Rust release").await?;
    println!("fast: {}", fast.content);

    let deep = client
        .search_deep("compare React vs Svelte performance")
        .await?;
    println!("deep providers: {:?}", deep.providers);

    let answer = client
        .search_answer("what is the capital of France?")
        .await?;
    println!("answer: {:?}", answer.answer);

    Ok(())
}
