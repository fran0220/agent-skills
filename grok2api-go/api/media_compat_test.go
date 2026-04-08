package api

import "testing"

func TestImageSizeToAspectRatioUsesSupportedRatios(t *testing.T) {
	tests := map[string]string{
		"1280x720":  "16:9",
		"720x1280":  "9:16",
		"1792x1024": "3:2",
		"1024x1792": "2:3",
		"1024x1024": "1:1",
	}

	for size, want := range tests {
		if got := imageSizeToAspectRatio(size); got != want {
			t.Fatalf("imageSizeToAspectRatio(%q) = %q, want %q", size, got, want)
		}
	}
}

func TestNormalizeVideoImageReferencesAcceptsOpenAIStyleBlocks(t *testing.T) {
	references, err := normalizeVideoImageReferences("https://legacy.example/extra.png", []any{
		map[string]any{
			"type": "image_url",
			"image_url": map[string]any{
				"url": "https://example.com/ref-1.png",
			},
		},
		"data:image/png;base64,Zm9v",
	})
	if err != nil {
		t.Fatalf("normalizeVideoImageReferences returned error: %v", err)
	}
	if len(references) != 3 {
		t.Fatalf("normalizeVideoImageReferences returned %d references, want 3", len(references))
	}
	if references[0] != "https://example.com/ref-1.png" {
		t.Fatalf("first reference = %q, want %q", references[0], "https://example.com/ref-1.png")
	}
	if references[1] != "data:image/png;base64,Zm9v" {
		t.Fatalf("second reference = %q, want data URI", references[1])
	}
	if references[2] != "https://legacy.example/extra.png" {
		t.Fatalf("third reference = %q, want legacy image field", references[2])
	}
}
