use crate::{error::StorageError, Serializer};

#[derive(Clone)]
pub struct WincodeSerializer;

impl<T> Serializer<T> for WincodeSerializer
where
    T: wincode::SchemaWrite<wincode::config::DefaultConfig, Src = T>
        + for<'de> wincode::SchemaRead<'de, wincode::config::DefaultConfig, Dst = T>,
{
    fn to_bytes(&self, value: &T) -> Result<Vec<u8>, crate::error::StorageError> {
        wincode::serialize(value).map_err(StorageError::WincodeWrite)
    }
    fn from_bytes(&self, bytes: &[u8]) -> Result<T, crate::error::StorageError> {
        wincode::deserialize(bytes).map_err(StorageError::WincodeRead)
    }
}
