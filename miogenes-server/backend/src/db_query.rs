use std::{collections::BTreeMap, sync::Arc};

use serde::de::DeserializeOwned;
use surrealdb::sql::Value;

use crate::MioState;

pub async fn select<T: DeserializeOwned>(
    state: Arc<MioState>,
    table: &str,
    query: Option<&str>,
    vars: Option<BTreeMap<String, Value>>,
) -> Result<T, anyhow::Error> {
    let mut query = state
        .db
        .execute(
            &format!("SELECT * FROM {table} {};", {
                if query.is_some() {
                    "WHERE ".to_owned() + query.unwrap()
                } else {
                    "".to_owned()
                }
            }),
            &state.sess,
            vars,
            false,
        )
        .await?;

    // terrible hack to serialize structs
    // serialize to_value into serde_json's own value system
    // then from_value the values generated
    Ok(serde_json::from_value({
        serde_json::to_value({
            query
                .pop()
                .ok_or(anyhow::anyhow!("query did not give response"))?
                .result?
        })?
    })?)
}
