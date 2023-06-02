use crate::error::MioInnerError;
use uuid::Uuid;

#[inline]
pub(crate) fn uuid_serialize(bytes: &Vec<u8>) -> Result<Uuid, MioInnerError> {
    Uuid::from_slice(bytes)
        .map_err(|err| MioInnerError::DbError(anyhow::anyhow!("failed to serialize uuid: {err}")))
}