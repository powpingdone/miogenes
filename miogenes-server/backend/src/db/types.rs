use chrono::{DateTime, Utc, Duration};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;

use super::Table;

pub struct Index<T: DbObject + DbTable> {
    id: Uuid,
    tbl: Table,
    inner: T,
}

impl<T> Index<T>
where
    T: DbObject + DbTable,
{
    pub fn new(id: Uuid, inner: &[u8]) -> Result<Self, T::Error> {
        Ok(Self {
            id,
            tbl: T::table(),
            inner: T::in_value(inner)?,
        })
    }

    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn inner_mut(&self) -> &mut T {
        &mut self.inner
    }

    pub fn consume(self) -> Result<Vec<u8>, T::Error> {
        self.inner.out_value()
    }
}

trait DbObject: Sized {
    type Error;

    fn in_value(raw: &[u8]) -> Result<Self, Self::Error>;
    fn out_value(self) -> Result<Vec<u8>, Self::Error>;
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

trait DbTable {
    const TABLE: Table;
    fn table() -> Table {
        Self::TABLE
    }
}

impl<T> DbTable for Index<T>
where
    T: DbTable + DbObject,
{
    const TABLE: Table = T::TABLE;
}

#[derive(Serialize, Deserialize)]
pub struct User {
    username: String,
    password: String,
    login_tokens: Vec<Uuid>,
}

impl DbTable for User {
    const TABLE: Table = Table::User;
}

#[derive(Serialize, Deserialize)]
pub struct UserToken {
    expiry: DateTime<Utc>,
    user: Uuid,
}

impl UserToken {
    pub fn push_forward(&mut self, t: Duration) {
        self.expiry += t;
    }

    pub fn is_expired(&self) -> bool {
        self.expiry < Utc::now()
    }
}

impl DbTable for UserToken {
    const TABLE: Table = Table::UserToken;
}
