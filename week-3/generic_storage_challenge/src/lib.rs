use error::StorageError;
use std::marker::PhantomData;

pub mod borsh_serializer;
pub mod error;
pub mod json_serializer;
pub mod wincode_serializer;

pub trait Serializer<T> {
    fn to_bytes(&self, value: &T) -> Result<Vec<u8>, StorageError>;
    fn from_bytes(&self, bytes: &[u8]) -> Result<T, StorageError>;
}

#[derive(Clone)]
pub struct Storage<T, S> {
    data: Option<Vec<u8>>,
    serializer: S,
    _phantom: PhantomData<T>,
}

impl<T, S> Storage<T, S>
where
    S: Serializer<T>,
{
    pub fn new(serializer: S) -> Self {
        Self {
            data: None,
            serializer,
            _phantom: PhantomData,
        }
    }

    pub fn save(&mut self, value: &T) -> Result<(), StorageError> {
        self.data = Some(self.serializer.to_bytes(value)?);
        Ok(())
    }

    pub fn load(&self) -> Result<T, StorageError> {
        match &self.data {
            Some(bytes) => self.serializer.from_bytes(bytes),
            None => Err(StorageError::NoData),
        }
    }

    pub fn has_data(&self) -> bool {
        self.data.is_some()
    }

    pub fn convert<NewS>(self, new_serializer: NewS) -> Result<Storage<T, NewS>, StorageError>
    where
        NewS: Serializer<T>,
    {
        let data = match self.data {
            Some(bytes) => {
                let value = self.serializer.from_bytes(&bytes)?;
                Some(new_serializer.to_bytes(&value)?)
            }
            None => None,
        };

        Ok(Storage {
            data,
            serializer: new_serializer,
            _phantom: PhantomData,
        })
    }
}
