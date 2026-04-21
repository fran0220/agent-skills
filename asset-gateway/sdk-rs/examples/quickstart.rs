use origin_asset::{AssetClient, AudioOptions, ImageOptions, TtsOptions};

#[tokio::main]
async fn main() -> origin_asset::Result<()> {
    let api_key = std::env::var("ORIGIN_API_KEY").expect("set ORIGIN_API_KEY env var");
    let client = AssetClient::new(api_key);

    let image = client
        .generate_image(
            "a medieval castle at sunset",
            Some(ImageOptions {
                size: Some("1792x1024".into()),
                transparent: Some(false),
                model: Some("gpt-image-1".into()),
                ..Default::default()
            }),
        )
        .await?;
    println!("Image: {:?}", image.output_url);

    let speech = client
        .generate_tts(
            "Welcome to the Origin platform!",
            Some(TtsOptions {
                voice: Some("alloy".into()),
                ..Default::default()
            }),
        )
        .await?;
    println!("TTS: {:?}", speech.output_url);

    let sfx = client
        .generate_audio(
            "sword clash metal impact",
            Some(AudioOptions {
                duration: Some(3),
                ..Default::default()
            }),
        )
        .await?;
    println!("SFX: {:?}", sfx.output_url);

    let providers = client.providers().await?;
    for provider in &providers {
        println!(
            "Provider: {} ({}) — {:?}",
            provider.display_name, provider.id, provider.asset_types
        );
    }

    Ok(())
}
