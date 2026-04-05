# chatgpt2api

Reverse proxy that converts ChatGPT web image generation (`chatgpt.com/images`) into OpenAI-compatible API endpoints.

Uses the same `POST /backend-api/f/conversation` endpoint with `system_hints: ["picture_v2"]` that the ChatGPT web UI uses — no model selector, no DALL-E API key needed. ChatGPT's backend decides which image model to use (currently gpt-image-1.5 / gpt-image-2).

## Quick Start

```bash
export ACCESS_TOKEN="your-chatgpt-access-token"
go run .
# listening on :8080
```

## Endpoints

### POST /v1/images/generations

Generate images from a text prompt. Compatible with OpenAI's image generation API.

```bash
curl -X POST http://localhost:8080/v1/images/generations \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "a cute cat on a space station, watercolor style",
    "n": 1,
    "size": "1024x1024",
    "response_format": "url"
  }'
```

Response:
```json
{
  "created": 1775366000,
  "data": [
    {
      "url": "https://files.oaiusercontent.com/...",
      "revised_prompt": "A cute orange tabby cat floating..."
    }
  ]
}
```

### POST /v1/images/edits

Edit an existing generated image (transformation or inpainting).

```bash
# Transformation (no mask — full image re-generation with reference)
curl -X POST http://localhost:8080/v1/images/edits \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "change the cat to a golden retriever",
    "image_file_id": "file_00000000dc7c71f5b1283eba162ff266",
    "gen_id": "beb94ced-6569-4438-bced-01ff1f571a24",
    "conversation_id": "69d1edac-bc34-83e8-adab-534411adf767",
    "parent_message_id": "8e9ca440-66cc-46fe-8ee1-e0822a6adacb"
  }'

# Inpainting (with mask — edit only the masked region)
curl -X POST http://localhost:8080/v1/images/edits \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "replace with a white kitten",
    "image_file_id": "file_00000000dc7c71f5b1283eba162ff266",
    "gen_id": "beb94ced-6569-4438-bced-01ff1f571a24",
    "mask_file_id": "file-FvT9Ggy5HuipM6qE7hZihr",
    "conversation_id": "69d1edac-bc34-83e8-adab-534411adf767",
    "parent_message_id": "8e9ca440-66cc-46fe-8ee1-e0822a6adacb"
  }'
```

## API Flow (captured from chatgpt.com/images)

```
1. POST /backend-api/sentinel/chat-requirements/prepare → get requirements token
2. POST /backend-api/f/conversation (SSE stream)
   - system_hints: ["picture_v2"]
   - model: "gpt-5-3" (auto-selected, not user-configurable)
   - SSE events contain multimodal_text parts with asset_pointer (file-service:// or sediment://)
3. GET /backend-api/files/download/{file_id}?conversation_id={conv_id} → { download_url }
4. Fetch download_url for actual image bytes
```

### Edit operations

| Operation | `metadata.dalle.from_client.operation` |
|-----------|---------------------------------------|
| Generate  | *(none)* |
| Transform | `{ "type": "transformation", "original_file_id": "...", "original_gen_id": "..." }` |
| Inpaint   | `{ "type": "inpainting", "original_file_id": "...", "original_gen_id": "...", "mask_file_id": "..." }` |

## Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `ACCESS_TOKEN` | Yes | — | ChatGPT access token (Bearer token from chatgpt.com) |
| `PORT` | No | `8080` | Listen port |
