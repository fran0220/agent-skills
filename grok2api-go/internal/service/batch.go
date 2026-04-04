package service

import (
	"context"
	"sync"
	"time"

	"github.com/google/uuid"
)

type BatchResult[T any] struct {
	OK        bool   `json:"ok"`
	Data      T      `json:"data,omitempty"`
	Error     string `json:"error,omitempty"`
	Cancelled bool   `json:"cancelled,omitempty"`
}

type BatchItemCallback[T any] func(item string, result BatchResult[T])

type BatchTask struct {
	ID        string
	Total     int
	Processed int
	OK        int
	Fail      int
	Status    string
	Warning   string
	Result    map[string]any
	Error     string
	CreatedAt time.Time

	mu         sync.Mutex
	queues     []chan map[string]any
	finalEvent map[string]any
	cancelled  bool
}

func NewBatchTask(total int) *BatchTask {
	return &BatchTask{
		ID:        uuid.NewString(),
		Total:     total,
		Status:    "running",
		CreatedAt: time.Now(),
	}
}

func (t *BatchTask) Snapshot() map[string]any {
	t.mu.Lock()
	defer t.mu.Unlock()
	return map[string]any{
		"task_id":   t.ID,
		"status":    t.Status,
		"total":     t.Total,
		"processed": t.Processed,
		"ok":        t.OK,
		"fail":      t.Fail,
		"warning":   t.Warning,
	}
}

func (t *BatchTask) Attach() chan map[string]any {
	q := make(chan map[string]any, 200)
	t.mu.Lock()
	t.queues = append(t.queues, q)
	t.mu.Unlock()
	return q
}

func (t *BatchTask) Detach(q chan map[string]any) {
	t.mu.Lock()
	defer t.mu.Unlock()
	for i, candidate := range t.queues {
		if candidate == q {
			t.queues = append(t.queues[:i], t.queues[i+1:]...)
			close(candidate)
			return
		}
	}
}

func (t *BatchTask) publish(event map[string]any) {
	t.mu.Lock()
	queues := append([]chan map[string]any(nil), t.queues...)
	t.mu.Unlock()
	for _, q := range queues {
		select {
		case q <- event:
		default:
		}
	}
}

func (t *BatchTask) Record(ok bool, item any, detail any, errMsg string) {
	t.mu.Lock()
	t.Processed++
	if ok {
		t.OK++
	} else {
		t.Fail++
	}
	event := map[string]any{
		"type":      "progress",
		"task_id":   t.ID,
		"total":     t.Total,
		"processed": t.Processed,
		"ok":        t.OK,
		"fail":      t.Fail,
	}
	if item != nil {
		event["item"] = item
	}
	if detail != nil {
		event["detail"] = detail
	}
	if errMsg != "" {
		event["error"] = errMsg
	}
	t.mu.Unlock()
	t.publish(event)
}

func (t *BatchTask) Finish(result map[string]any, warning string) {
	t.mu.Lock()
	t.Status = "done"
	t.Warning = warning
	t.Result = result
	event := map[string]any{
		"type":      "done",
		"task_id":   t.ID,
		"total":     t.Total,
		"processed": t.Processed,
		"ok":        t.OK,
		"fail":      t.Fail,
		"warning":   warning,
		"result":    result,
	}
	t.finalEvent = event
	t.mu.Unlock()
	t.publish(event)
}

func (t *BatchTask) FailTask(errMsg string) {
	t.mu.Lock()
	t.Status = "error"
	t.Error = errMsg
	event := map[string]any{
		"type":      "error",
		"task_id":   t.ID,
		"total":     t.Total,
		"processed": t.Processed,
		"ok":        t.OK,
		"fail":      t.Fail,
		"error":     errMsg,
	}
	t.finalEvent = event
	t.mu.Unlock()
	t.publish(event)
}

func (t *BatchTask) Cancel() {
	t.mu.Lock()
	t.cancelled = true
	t.mu.Unlock()
}

func (t *BatchTask) Cancelled() bool {
	t.mu.Lock()
	defer t.mu.Unlock()
	return t.cancelled
}

func (t *BatchTask) FinishCancelled() {
	t.mu.Lock()
	t.Status = "cancelled"
	event := map[string]any{
		"type":      "cancelled",
		"task_id":   t.ID,
		"total":     t.Total,
		"processed": t.Processed,
		"ok":        t.OK,
		"fail":      t.Fail,
	}
	t.finalEvent = event
	t.mu.Unlock()
	t.publish(event)
}

func (t *BatchTask) FinalEvent() map[string]any {
	t.mu.Lock()
	defer t.mu.Unlock()
	if t.finalEvent == nil {
		return nil
	}
	copy := make(map[string]any, len(t.finalEvent))
	for key, value := range t.finalEvent {
		copy[key] = value
	}
	return copy
}

func RunBatch[T any](ctx context.Context, items []string, worker func(context.Context, string) (T, error), batchSize int, task *BatchTask, onItem BatchItemCallback[T], shouldCancel func() bool) map[string]BatchResult[T] {
	if batchSize <= 0 {
		batchSize = 50
	}
	results := make(map[string]BatchResult[T], len(items))
	var mu sync.Mutex

	for start := 0; start < len(items); start += batchSize {
		if ctx.Err() != nil || (task != nil && task.Cancelled()) || (shouldCancel != nil && shouldCancel()) {
			break
		}
		end := start + batchSize
		if end > len(items) {
			end = len(items)
		}
		chunk := items[start:end]
		var wg sync.WaitGroup
		for _, item := range chunk {
			item := item
			wg.Add(1)
			go func() {
				defer wg.Done()
				var zero T
				if ctx.Err() != nil || (task != nil && task.Cancelled()) || (shouldCancel != nil && shouldCancel()) {
					result := BatchResult[T]{OK: false, Error: "cancelled", Cancelled: true, Data: zero}
					mu.Lock()
					results[item] = result
					mu.Unlock()
					return
				}
				data, err := worker(ctx, item)
				result := BatchResult[T]{OK: err == nil, Data: data}
				if err != nil {
					result.Error = err.Error()
				}
				mu.Lock()
				results[item] = result
				mu.Unlock()
				if task != nil {
					task.Record(result.OK, item, result, result.Error)
				}
				if onItem != nil {
					onItem(item, result)
				}
			}()
		}
		wg.Wait()
	}

	return results
}
