use borsh::{BorshDeserialize, BorshSerialize};
use std::collections::VecDeque;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct Todo {
    pub id: u64,
    pub description: String,
    pub created_at: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Queue<T> {
    pub items: VecDeque<T>,
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Self {
            items: VecDeque::new(),
        }
    }

    pub fn enqueue(&mut self, item: T) {
        self.items.push_back(item);
    }

    pub fn dequeue(&mut self) -> Option<T> {
        self.items.pop_front()
    }

    pub fn peek(&self) -> Option<&T> {
        self.items.front()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn remove(&mut self, index: usize) -> Option<T> {
        self.items.remove(index)
    }
}

impl<T> Queue<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    pub fn save(&self, path: &str) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        borsh::to_writer(&mut file, self)?;
        Ok(())
    }

    pub fn load(path: &str) -> std::io::Result<Self> {
        if !Path::new(path).exists() {
            return Ok(Self::new());
        }
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        if buffer.is_empty() {
            return Ok(Self::new());
        }

        match borsh::from_slice(&buffer) {
            Ok(queue) => Ok(queue),
            Err(_) => Ok(Self::new()),
        }
    }
}
