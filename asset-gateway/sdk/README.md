# @assetforge/sdk

```bash
npm install @assetforge/sdk
```

```ts
import { AssetForge } from "@assetforge/sdk";

const forge = new AssetForge({
  apiKey: "agk_...",
  baseUrl: "https://upload.xiaomao.chat",
});
```

## Generate

```ts
const image = await forge.image("cinematic sci-fi skyline", {
  size: "1536x1024",
  transparent: true,
});

const video = await forge.video({
  prompt: "slow camera pan",
  input: image.url!,
});

const music = await forge.music("uplifting synth theme", { duration: 30 });
const audio = await forge.audio("explosion sfx", { type: "sfx" });
const tts = await forge.tts("Hello world", { voice: "Cherry" });
const model = await forge.model3d("treasure chest");
const sprite = await forge.sprite("knight walk cycle", {
  animationType: "walk",
  duration: 2,
});
const world = await forge.world("misty medieval village");
const text = await forge.text("Write a short fantasy intro");
```

```ts
const raw = await forge.generate({
  asset_type: "image",
  prompt: "minimal icon set",
  size: "1024x1024",
  transparent: true,
});
```

## Jobs

```ts
const jobs = await forge.job.list({ status: "completed", limit: 20 });
const job = await forge.job.status("job-id");
await forge.job.cancel("job-id");
```

## Stream

```ts
const stream = forge.stream.image("epic landscape");

stream.on("progress", (event) => {
  console.log(event.status, event.percent);
});

stream.on("done", (result) => {
  console.log(result.url);
});

stream.on("error", (error) => {
  console.error(error.code, error.message);
});
```

```ts
const tracked = forge.stream.job("job-id");
tracked.on("progress", (event) => console.log(event.job?.status));
```

## Upload + Assets

```ts
const uploaded = await forge.upload(file);
const files = await forge.assets.list();
await forge.assets.delete(uploaded.filename);
```

## Providers

```ts
const providers = await forge.providers.list();
const health = await forge.providers.health();
const gemini = await forge.providers.health("gemini_image");
```

## Process

```ts
const processed = await forge.process({
  input: image.url!,
  operations: [
    { op: "smart_crop", mode: "tightest" },
    { op: "resize", width: 512, height: 512 },
  ],
});

const rigged = await forge.process3d({
  task_id: "tripo-task-id",
  operation: "rig",
});
```

## Voice

```ts
await forge.voice.clone({
  name: "hero",
  audio_base64: "...",
  audio_mime: "audio/mpeg",
});

await forge.voice.design({
  name: "narrator",
  voice_prompt: "warm documentary narrator",
  preview_text: "Welcome to AssetForge.",
});

const voices = await forge.voice.list({ type: "vc" });
```

## Errors

```ts
import { AssetForgeError } from "@assetforge/sdk";

try {
  await forge.image("...");
} catch (error) {
  const e = error as AssetForgeError;
  console.error(e.code, e.message);
}
```
