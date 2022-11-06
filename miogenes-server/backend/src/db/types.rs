use serde::{de::DeserializeOwned, Serialize};
use uuid::Uuid;

// trait impl for ser/deser
pub trait DbObject: Sized {
    type Error;

    fn in_value(raw: &[u8]) -> Result<Self, Self::Error>;
    fn out_value(self) -> Result<Vec<u8>, Self::Error>;
}

// table identifier
pub trait DbTable {
    type Ret;
    const TABLE: Self::Ret;
    fn table() -> Self::Ret {
        Self::TABLE
    }
}

#[derive(Clone)]
pub struct Index<T: DbObject + DbTable + Send + Clone> {
    id: Uuid,
    tbl: T::Ret,
    inner: T,
}

impl<T> Index<T>
where
    T: DbObject + DbTable + Send + Clone,
{
    pub fn new(id: Uuid, inner: &[u8]) -> Result<Self, T::Error> {
        Ok(Self {
            id,
            tbl: T::table(),
            inner: T::in_value(inner)?,
        })
    }

    pub fn id(&self)-> Uuid {
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


impl<T> DbTable for Index<T>
where
    T: DbTable + DbObject + Send + Clone,
{
    type Ret = T::Ret;
    const TABLE: T::Ret = T::TABLE;
}
