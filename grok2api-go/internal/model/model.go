package model

import "fmt"

type Tier string

const (
	TierBasic Tier = "basic"
	TierSuper Tier = "super"
)

type Cost string

const (
	CostLow  Cost = "low"
	CostHigh Cost = "high"
)

type ModelInfo struct {
	ModelID     string `json:"model_id"`
	GrokModel   string `json:"grok_model"`
	ModelMode   string `json:"model_mode"`
	Tier        Tier   `json:"tier"`
	Cost        Cost   `json:"cost"`
	DisplayName string `json:"display_name"`
	Description string `json:"description,omitempty"`
	IsImage     bool   `json:"is_image"`
	IsImageEdit bool   `json:"is_image_edit"`
	IsVideo     bool   `json:"is_video"`
}

type Service struct {
	models []ModelInfo
	byID   map[string]ModelInfo
}

func NewService() *Service {
	models := []ModelInfo{
		{ModelID: "grok-3", GrokModel: "grok-3", ModelMode: "MODEL_MODE_GROK_3", Tier: TierBasic, Cost: CostLow, DisplayName: "GROK-3"},
		{ModelID: "grok-3-mini", GrokModel: "grok-3", ModelMode: "MODEL_MODE_GROK_3_MINI_THINKING", Tier: TierBasic, Cost: CostLow, DisplayName: "GROK-3-MINI"},
		{ModelID: "grok-3-thinking", GrokModel: "grok-3", ModelMode: "MODEL_MODE_GROK_3_THINKING", Tier: TierBasic, Cost: CostLow, DisplayName: "GROK-3-THINKING"},
		{ModelID: "grok-4", GrokModel: "grok-4", ModelMode: "MODEL_MODE_GROK_4", Tier: TierBasic, Cost: CostLow, DisplayName: "GROK-4"},
		{ModelID: "grok-4-thinking", GrokModel: "grok-4", ModelMode: "MODEL_MODE_GROK_4_THINKING", Tier: TierBasic, Cost: CostLow, DisplayName: "GROK-4-THINKING"},
		{ModelID: "grok-4-heavy", GrokModel: "grok-4", ModelMode: "MODEL_MODE_HEAVY", Tier: TierSuper, Cost: CostHigh, DisplayName: "GROK-4-HEAVY"},
		{ModelID: "grok-4.1-mini", GrokModel: "grok-4-1-thinking-1129", ModelMode: "MODEL_MODE_GROK_4_1_MINI_THINKING", Tier: TierBasic, Cost: CostLow, DisplayName: "GROK-4.1-MINI"},
		{ModelID: "grok-4.1-fast", GrokModel: "grok-4-1-thinking-1129", ModelMode: "MODEL_MODE_FAST", Tier: TierBasic, Cost: CostLow, DisplayName: "GROK-4.1-FAST"},
		{ModelID: "grok-4.1-expert", GrokModel: "grok-4-1-thinking-1129", ModelMode: "MODEL_MODE_EXPERT", Tier: TierBasic, Cost: CostHigh, DisplayName: "GROK-4.1-EXPERT"},
		{ModelID: "grok-4.1-thinking", GrokModel: "grok-4-1-thinking-1129", ModelMode: "MODEL_MODE_GROK_4_1_THINKING", Tier: TierBasic, Cost: CostHigh, DisplayName: "GROK-4.1-THINKING"},
		{ModelID: "grok-4.20-beta", GrokModel: "grok-420", ModelMode: "MODEL_MODE_GROK_420", Tier: TierBasic, Cost: CostLow, DisplayName: "GROK-4.20-BETA"},
		{ModelID: "grok-imagine-1.0-fast", GrokModel: "grok-3", ModelMode: "MODEL_MODE_FAST", Tier: TierBasic, Cost: CostHigh, DisplayName: "Grok Image Fast", Description: "Imagine waterfall image generation model for chat completions", IsImage: true},
		{ModelID: "grok-imagine-1.0", GrokModel: "grok-3", ModelMode: "MODEL_MODE_FAST", Tier: TierBasic, Cost: CostHigh, DisplayName: "Grok Image", Description: "Image generation model", IsImage: true},
		{ModelID: "grok-imagine-1.0-edit", GrokModel: "imagine-image-edit", ModelMode: "MODEL_MODE_FAST", Tier: TierBasic, Cost: CostHigh, DisplayName: "Grok Image Edit", Description: "Image edit model", IsImageEdit: true},
		{ModelID: "grok-imagine-1.0-video", GrokModel: "grok-3", ModelMode: "MODEL_MODE_FAST", Tier: TierSuper, Cost: CostHigh, DisplayName: "Grok Video", Description: "Video generation model", IsVideo: true},
	}

	byID := make(map[string]ModelInfo, len(models))
	for _, item := range models {
		byID[item.ModelID] = item
	}

	return &Service{models: models, byID: byID}
}

func (s *Service) Get(modelID string) (ModelInfo, bool) {
	model, ok := s.byID[modelID]
	return model, ok
}

func (s *Service) List() []ModelInfo {
	result := make([]ModelInfo, len(s.models))
	copy(result, s.models)
	return result
}

func (s *Service) Valid(modelID string) bool {
	_, ok := s.byID[modelID]
	return ok
}

func (s *Service) ToGrok(modelID string) (string, string, error) {
	model, ok := s.Get(modelID)
	if !ok {
		return "", "", fmt.Errorf("invalid model ID: %s", modelID)
	}
	return model.GrokModel, model.ModelMode, nil
}

func (s *Service) PoolForModel(modelID string) string {
	model, ok := s.Get(modelID)
	if ok && model.Tier == TierSuper {
		return "ssoSuper"
	}
	return "ssoBasic"
}

func (s *Service) PoolCandidatesForModel(modelID string) []string {
	model, ok := s.Get(modelID)
	if ok && model.Tier == TierSuper {
		return []string{"ssoSuper"}
	}
	// 基础模型仅使用 basic 池，不回退到 super 池（避免浪费 super 额度）
	return []string{"ssoBasic"}
}
