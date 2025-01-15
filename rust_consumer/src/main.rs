use cosmos_driver::{Pipeline, PipelineItem};
use serde_json::json;

fn main() {
    let mut pipeline = Pipeline::new(vec!["p1".to_string(), "p2".to_string()]);

    pipeline
        .enqueue_items(
            "p1",
            vec![
                PipelineItem::from(
                    json!({"order": 2, "payload": {"id": 1, "title": "Partition 1 / Item 1 / Order 2"}}),
                ),
                PipelineItem::from(
                    json!({"order": 1, "payload": {"id": 2, "title": "Partition 1 / Item 2 / Order 1"}}),
                ),
                PipelineItem::from(
                    json!({"order": 2, "payload": {"id": 3, "title": "Partition 1 / Item 3 / Order 2"}}),
                ),
                PipelineItem::from(
                    json!({"order": 1, "payload": {"id": 4, "title": "Partition 1 / Item 4 / Order 1"}}),
                ),
                PipelineItem::from(
                    json!({"order": 2, "payload": {"id": 5, "title": "Partition 1 / Item 5 / Order 2"}}),
                ),
            ],
        )
        .unwrap();
    pipeline
        .enqueue_items(
            "p2",
            vec![
                PipelineItem::from(
                    json!({"order": 1, "payload": {"id": 6, "title": "Partition 2 / Item 1 / Order 1"}}),
                ),
                PipelineItem::from(
                    json!({"order": 2, "payload": {"id": 7, "title": "Partition 2 / Item 2 / Order 2"}}),
                ),
                PipelineItem::from(
                    json!({"order": 1, "payload": {"id": 8, "title": "Partition 2 / Item 3 / Order 1"}}),
                ),
                PipelineItem::from(
                    json!({"order": 2, "payload": {"id": 9, "title": "Partition 2 / Item 4 / Order 2"}}),
                ),
                PipelineItem::from(
                    json!({"order": 1, "payload": {"id": 10, "title": "Partition 2 / Item 5 / Order 1"}}),
                ),
            ],
        )
        .unwrap();

    while let Some(i) = pipeline.next_item() {
        println!("Recieved item: {:#?}", i);
    }
}
