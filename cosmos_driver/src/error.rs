use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    #[error("partition does not exist")]
    PartitionDoesNotExist,
}
