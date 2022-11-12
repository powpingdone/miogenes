use serde::{de::DeserializeOwned, Serialize};
use uuid::Uuid;

use std::fmt::Debug;

// trait impl for ser/deser
pub trait DbObject: Sized {
    type Error;

    fn in_value(raw: &[u8]) -> Result<Self, Self::Error>;
    fn out_value(self) -> Result<Vec<u8>, Self::Error>;
}

// table to bytes that uniquely identifies it
pub trait DbTable {
    fn table(&self) -> Box<[u8]>;
}

impl<T> Index<T>
where
    T: DbObject + DbTable + Send + Clone + Debug,
{
    fn table(&self) -> Box<[u8]> {
        self.inner.table()
    }
}

#[derive(Clone, Debug)]
pub struct Index<T: DbObject + DbTable + Send + Clone + Debug> {
    id: Uuid,
    inner: T,
}

impl<T> Index<T>
where
    T: DbObject + DbTable + Send + Clone + Debug,
{
    pub fn new(id: Uuid, inner: &[u8]) -> Result<Self, T::Error> {
        Ok(Self {
            id,
            inner: T::in_value(inner)?,
        })
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    pub fn decompose(self) -> Result<Vec<u8>, T::Error> {
        self.inner.out_value()
    }
}

impl<T> DbObject for T
where
    T: DeserializeOwned + Serialize + Sized,
{
    fn in_value(raw: &[u8]) -> Result<T, serde_json::Error> {
        serde_json::from_slice(raw)
    }

    fn out_value(self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(&self)
    }

    type Error = serde_json::Error;
}
