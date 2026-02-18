use crate::{error::StorageError, Serializer};
#[derive(Clone)]
pub struct JsonSerializer;

impl<T> Serializer<T> for JsonSerializer
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    fn to_bytes(&self, value: &T) -> Result<Vec<u8>, crate::error::StorageError> {
        serde_json::to_vec(value).map_err(StorageError::Json)
    }

    fn from_bytes(&self, bytes: &[u8]) -> Result<T, crate::error::StorageError> {
        serde_json::from_slice(bytes).map_err(StorageError::Json)
    }
}
