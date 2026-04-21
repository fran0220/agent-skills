use origin_gamedb::{CognifyOptions, GameDbClient, SearchOptions, SearchType};

#[tokio::main]
async fn main() -> origin_gamedb::Result<()> {
    let gamedb = GameDbClient::new("ca_xxx_token");

    gamedb.create_dataset("combat-design").await?;
    gamedb
        .add_text(
            "combat-design",
            "Shield parries briefly expose enemies to critical hits.",
        )
        .await?;

    gamedb
        .cognify(&CognifyOptions {
            datasets: Some(vec!["combat-design".into()]),
            run_in_background: Some(true),
            ..Default::default()
        })
        .await?;

    let results = gamedb
        .search(
            "How do parries create combat openings?",
            Some(SearchOptions {
                search_type: Some(SearchType::Summaries),
                top_k: Some(5),
                datasets: Some(vec!["combat-design".into()]),
            }),
        )
        .await?;

    println!("{results}");
    Ok(())
}
