use super::{TopLevel, DbTable};

pub fn migrate(db: &sled::Db) {
    let migration = db.open_tree("migration").unwrap();

    // v1 migration 
    if migration.contains_key(b"v1").unwrap() {
        migrate_v1(db);
        migration.insert(b"v1", b"").unwrap().unwrap();
    }
}

fn migrate_v1(db: &sled::Db) {
    db.open_tree(TopLevel::User.table()).unwrap();
    db.open_tree(TopLevel::UserToken.table()).unwrap();
    db.open_tree(TopLevel::IndexUsernameToUser.table()).unwrap();
}