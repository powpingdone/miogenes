use axum::Json;
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

// table to bytes that is id'd
pub trait IdTable {
    fn id_table(&self, id: Uuid) -> Box<[u8]>;
}

impl<T> Index<T>
where
    T: DbObject + DbTable + Send + Clone + Debug,
{
    fn table(&self) -> Box<[u8]> {
        self.inner.table()
    }
}

impl<T> Index<T>
where
    T: DbObject + IdTable + Send + Clone + Debug,
{
    fn id_table(&self) -> Box<[u8]> {
        self.inner.id_table(self.owner.unwrap())
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Index<T: DbObject + Send + Clone + Debug> {
    id: Uuid,
    inner: T,
    #[serde(skip)]
    owner: Option<Uuid>,
}

impl<T> Index<T>
where
    T: DbObject + DbTable + Send + Clone + Debug,
{
    pub fn new(id: Uuid, inner: &[u8]) -> Result<Self, T::Error> {
        Ok(Self {
            id,
            inner: T::in_value(inner)?,
            owner: None,
        })
    }
}

impl<T> Index<T>
where
    T: DbObject + IdTable + Send + Clone + Debug,
{
    pub fn new_owned(id: Uuid, table_id: Uuid, inner: &[u8]) -> Result<Self, T::Error> {
        Ok(Self {
            id,
            inner: T::in_value(inner)?,
            owner: Some(table_id),
        })
    }
}

impl<T> Index<T>
where
    T: DbObject + Send + Clone + Debug,
{
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    pub fn inner_owned(self) -> T {
        self.inner
    }

    pub fn decompose(self) -> Result<Vec<u8>, T::Error> {
        self.inner.out_value()
    }
}

impl<T> Index<T>
where
    T: DbObject + Send + Clone + Debug + Serialize,
{
    pub fn web_out(self) -> Json<Self> {
        Json(self)
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
