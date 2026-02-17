use std::fmt::Debug;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Borsh error: {0}")]
    Borsh(#[from] std::io::Error),

    #[error("Wincode write error: {0}")]
    WincodeWrite(#[from] wincode::WriteError),

    #[error("Wincode read error: {0}")]
    WincodeRead(#[from] wincode::ReadError),

    #[error("Json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("No data stored")]
    NoData,
}
