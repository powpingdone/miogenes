use std::sync::Arc;

use log::*;
use surrealdb::{Datastore, Session};

pub async fn migrate(db: Arc<Datastore>) {
    debug!("running migrations");

    // define ns and db
    if db
        .execute(
            "USE NS ns DB db; INFO FOR DB;",
            &Session::for_kv(),
            None,
            false,
        )
        .await
        .unwrap()[0]
        .result
        .is_err()
    {
        // if the ns doesnt exist
        trace!(
            "{:?}",
            db.execute(
                "DEFINE NS ns; DEFINE DB db;",
                &Session::for_kv(),
                None,
                false,
            )
            .await
            .unwrap()
        );
    }
    let s = Session::for_db("ns", "db");

    // define user table
    if db
        .execute("INFO FOR TABLE user;", &s, None, false)
        .await
        .unwrap()[0]
        .result
        .is_err()
    {
        trace!(
            "{:?}",
            db.execute(
                r#"
        DEFINE TABLE user;
        DEFINE FIELD userid ON user TYPE uuid;
        DEFINE FIELD password ON user TYPE string;
        DEFINE FIELD username ON user TYPE string;
        "#,
                &s,
                None,
                false
            )
            .await
            .unwrap()
        );
    }
}
