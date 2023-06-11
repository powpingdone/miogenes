use crate::error::MioInnerError;
use sqlx::prelude::*;
use sqlx::SqliteConnection;
use std::future::Future;
use std::pin::Pin;
use uuid::Uuid;

// small function for serialization of uuids from the database
#[inline]
pub(crate) fn uuid_serialize(bytes: &[u8]) -> Result<Uuid, MioInnerError> {
    Uuid::from_slice(bytes)
        .map_err(|err| MioInnerError::DbError(anyhow::anyhow!("failed to serialize uuid: {err}")))
}

// function for using sqlite BEGIN IMMEDIATE for writing out a database in a txn
pub(crate) fn write_transaction<'a, Good, Txn>(
    conn: &'a mut SqliteConnection,
    f: Txn,
) -> Pin<Box<dyn Future<Output = Result<Good, MioInnerError>> + Send + 'a>>
where
    Txn: for<'c> FnOnce(
            &'c mut SqliteConnection,
        )
            -> Pin<Box<dyn Future<Output = Result<Good, MioInnerError>> + Send + 'c>>
        + 'a
        + Send
        + Sync,
    Good: Send,
{
    Box::pin(async move {
        conn.execute("BEGIN IMMEDIATE;").await?;
        match f(conn).await {
            Ok(ok) => {
                conn.execute("COMMIT;").await?;
                Ok(ok)
            }
            Err(err) => {
                conn.execute("ROLLBACK;").await?;
                Err(err)
            }
        }
    })
}
