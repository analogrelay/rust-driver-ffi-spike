package main

import (
	"encoding/json"
	"fmt"
	"strings"
	"testing"

	"github.com/analogrelay/go-rust-interop/go_consumer/pipeline"
)

// Generate a very deep JSON object
func generateDeepJSON(depth int) string {
	if depth == 0 {
		return `"value"`
	}
	var builder strings.Builder
	builder.WriteString("[")
	for i := 0; i < 10; i++ {
		if i != 0 {
			builder.WriteString(",")
		}
		builder.WriteString(fmt.Sprintf(`{"key":%s}`, generateDeepJSON(depth-1)))
	}
	builder.WriteString("]")
	return builder.String()
}

func createItems(partitionId int, count int, startId int) ([]byte, error) {
	items := make([]map[string]interface{}, count)
	order := partitionId - 1
	for i := 0; i < count; i++ {
		item := map[string]interface{}{
			"order": order + 1,
			"payload": Item{
				id:       fmt.Sprint(startId + i),
				title:    fmt.Sprintf("Partition %d / Item %d / Order %d", partitionId, i, order+1),
				hugeData: json.RawMessage(generateDeepJSON(7)),
			},
		}
		items[i] = item

		order = (order % 2) + 1
	}
	return json.Marshal(items)
}

type Item struct {
	id       string
	title    string
	hugeData json.RawMessage
}

func BenchmarkPipeline(b *testing.B) {
	partitions := []string{
		"p1",
		"p2",
	}
	data := make(map[string][]byte)
	for idx := range partitions {
		item, err := createItems(idx+1, 5, idx*5+1)
		if err != nil {
			panic(err)
		}
		data[partitions[idx]] = item
	}

	for i := 0; i < b.N; i++ {
		pipeline, err := pipeline.NewPipeline[map[string]Item](partitions)
		if err != nil {
			panic(err)
		}
		defer pipeline.Free()

		for _, partition := range partitions {
			err = pipeline.EnqueueData(partition, data[partition])
			if err != nil {
				panic(err)
			}
		}

		for _, err := range pipeline.Iter() {
			if err != nil {
				panic(err)
			}
		}
	}
}
