use std::collections::{BTreeMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::Error;

#[cfg(not(feature = "raw_value"))]
pub type PipelinePayload = serde_json::Value;
#[cfg(feature = "raw_value")]
pub type PipelinePayload = Box<serde_json::value::RawValue>;

#[derive(Debug, Deserialize, Serialize)]
pub struct PipelineItem {
    pub order: usize,
    pub payload: PipelinePayload,
}

impl From<serde_json::Value> for PipelineItem {
    fn from(value: serde_json::Value) -> Self {
        serde_json::from_value(value).unwrap()
    }
}

struct PartitionState {
    #[allow(dead_code)]
    partition_id: String,
    buffer: VecDeque<PipelineItem>,
}

#[allow(dead_code)]
enum ReadStrategy {
    RoundRobin { next: usize },
    Ordered,
}

impl ReadStrategy {
    pub fn select(
        &mut self,
        partitions: &mut BTreeMap<String, PartitionState>,
    ) -> Option<PipelinePayload> {
        match self {
            Self::RoundRobin { next } => {
                let partition = partitions.values_mut().nth(*next)?;
                let item = partition.buffer.pop_front()?;
                *next = (*next + 1) % partitions.len();
                Some(item.payload)
            }
            Self::Ordered => {
                let partition = {
                    let mut min_order = usize::MAX;
                    let mut min_partition = None;
                    for (_, partition) in partitions.iter_mut() {
                        if let Some(item) = partition.buffer.front() {
                            if item.order < min_order {
                                min_order = item.order;
                                min_partition = Some(partition);
                            }
                        }
                    }
                    min_partition
                };
                partition
                    .and_then(|p| p.buffer.pop_front())
                    .map(|i| i.payload)
            }
        }
    }
}

/// The pipeline component, which is the main object created and used by the consumer code.
pub struct Pipeline {
    partitions: BTreeMap<String, PartitionState>,
    partition_read_strategy: ReadStrategy,
}

impl Pipeline {
    pub fn new(partition_ids: Vec<String>) -> Self {
        let partitions = partition_ids
            .into_iter()
            .map(|id| {
                (
                    id.clone(),
                    PartitionState {
                        partition_id: id,
                        buffer: VecDeque::new(),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        Self {
            partitions,
            partition_read_strategy: ReadStrategy::Ordered,
        }
    }

    pub fn enqueue_items_raw(&mut self, partition_id: &str, buffer: &[u8]) -> Result<(), Error> {
        // Parse the buffer as JSON
        let items: Vec<PipelineItem> = serde_json::from_slice(buffer)?;
        self.enqueue_items(partition_id, items)
    }

    pub fn enqueue_items(
        &mut self,
        partition_id: &str,
        items: Vec<PipelineItem>,
    ) -> Result<(), Error> {
        let partition = self
            .partitions
            .get_mut(partition_id)
            .ok_or(Error::PartitionDoesNotExist)?;
        partition.buffer.extend(items);
        Ok(())
    }

    pub fn next_item(&mut self) -> Option<PipelinePayload> {
        self.partition_read_strategy.select(&mut self.partitions)
    }

    pub fn next_item_raw(&mut self) -> Result<Option<Box<[u8]>>, Error> {
        let Some(item) = self.next_item() else {
            return Ok(None);
        };
        let item = serde_json::to_vec(&item)?;
        Ok(Some(item.into_boxed_slice()))
    }
}
