use std::sync::Arc;

use log::*;
use surrealdb::{Datastore, Session};

pub async fn migrate(db: Arc<Datastore>) {
    debug!("running migrations");

    // define ns and db
    if let Err(_) = db
        .execute("USE NS ns DB db; INFO FOR DB;", &Session::for_kv(), None, false)
        .await
        .unwrap()[0]
        .result
    {
        // if the ns doesnt exist
        trace!("{:?}", db.execute(
            "DEFINE NS ns; DEFINE DB db;",
            &Session::for_kv(),
            None,
            false,
        )
        .await
        .unwrap());
    }
    let s = Session::for_db("ns", "db");

    // define user table
    if let Err(_) = db
        .execute("INFO FOR TABLE user;", &s, None, false)
        .await
        .unwrap()[0]
        .result
    {
        trace!(
            "{:?}",
            db.execute(
                r#"
        DEFINE TABLE user;
        DEFINE FIELD id ON user TYPE uuid;
        DEFINE FIELD password ON user TYPE array;
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
