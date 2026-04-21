use origin_asset::asset::{AudioOptions, ImageOptions, TtsOptions};
use origin_asset::OriginClient;

#[tokio::main]
async fn main() -> origin_asset::Result<()> {
    let api_key = std::env::var("ORIGIN_API_KEY").expect("set ORIGIN_API_KEY env var");
    let client = OriginClient::new(api_key);
    let asset = client.asset();

    // Generate an image with options
    let image = asset
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

    // Generate TTS
    let speech = asset
        .generate_tts(
            "Welcome to the Origin platform!",
            Some(TtsOptions {
                voice: Some("alloy".into()),
                ..Default::default()
            }),
        )
        .await?;
    println!("TTS: {:?}", speech.output_url);

    // Generate sound effects
    let sfx = asset
        .generate_audio(
            "sword clash metal impact",
            Some(AudioOptions {
                duration: Some(3),
                ..Default::default()
            }),
        )
        .await?;
    println!("SFX: {:?}", sfx.output_url);

    // List providers
    let providers = asset.providers().await?;
    for p in &providers {
        println!(
            "Provider: {} ({}) — {:?}",
            p.display_name, p.id, p.asset_types
        );
    }

    Ok(())
}
