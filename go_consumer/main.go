package main

import (
	"encoding/json"
	"fmt"
	"strings"
	"time"

	"github.com/analogrelay/go-rust-interop/go_consumer/pipeline"
)

// Generate a very deep JSON object
func generateDeepJSON(depth int) string {
	if depth == 0 {
		return `"value"`
	}
	var builder strings.Builder
	builder.WriteString("[")
	for i := 0; i < 2; i++ {
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
				Id:       fmt.Sprint(startId + i),
				Title:    fmt.Sprintf("Partition %d / Item %d / Order %d", partitionId, i, order+1),
				HugeData: json.RawMessage(generateDeepJSON(2)),
			},
		}
		items[i] = item

		order = (order % 2) + 1
	}
	j, _ := json.Marshal(items)
	fmt.Println(string(j))
	return j, nil
}

type Item struct {
	Id       string          `json:"id"`
	Title    string          `json:"title"`
	HugeData json.RawMessage `json:"huge_data"`
}

func main() {
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

	fmt.Println("Go: Creating pipeline")
	pipeline, err := pipeline.NewPipeline[Item](partitions)
	if err != nil {
		panic(err)
	}
	defer func() {
		fmt.Println("Go: Freeing pipeline")
		pipeline.Free()
	}()

	for _, partition := range partitions {
		start := time.Now()
		err = pipeline.EnqueueData(partition, data[partition])
		fmt.Println("Go: Enqueued data for partition", partition, "in", time.Since(start))
		if err != nil {
			panic(err)
		}
	}

	fmt.Println("Go: Iterating pipeline")
	for item, err := range pipeline.Iter() {
		if err != nil {
			panic(err)
		}
		fmt.Println("Go: Received Item", item.Title)
	}
}
