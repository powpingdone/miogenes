use super::Table;

pub fn migrate(db: &sled::Db) {
    let migration = db.open_tree("migration").unwrap();

    // v1 migration 
    if migration.contains_key(b"v1").unwrap() {
        migrate_v1(db);
        migration.insert(b"v1", b"");
    }
}

fn migrate_v1(db: &sled::Db) {
    db.open_tree(Table::User).unwrap();
    db.open_tree(Table::UserToken).unwrap();
    db.open_tree(Table::Album).unwrap();
    db.open_tree(Table::AlbumArt).unwrap();
    db.open_tree(Table::Artist).unwrap();
    db.open_tree(Table::Tracks).unwrap();
}