use crate::{error::StorageError, Serializer};

pub struct BorshSerializer;

impl<T> Serializer<T> for BorshSerializer
where
    T: borsh::BorshSerialize + borsh::BorshDeserialize,
{
    fn to_bytes(&self, value: &T) -> Result<Vec<u8>, crate::error::StorageError> {
        borsh::to_vec(value).map_err(StorageError::Borsh)
    }
    fn from_bytes(&self, bytes: &[u8]) -> Result<T, StorageError> {
        borsh::from_slice(bytes).map_err(StorageError::Borsh)
    }
}
