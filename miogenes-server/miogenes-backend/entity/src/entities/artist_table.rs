//! SeaORM Entity. Generated by sea-orm-codegen 0.9.1

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "artist_table")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub ts: i64,
    pub name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::album_table::Entity")]
    AlbumTable,
    #[sea_orm(has_many = "super::track_table::Entity")]
    TrackTable,
}

impl Related<super::album_table::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AlbumTable.def()
    }
}

impl Related<super::track_table::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TrackTable.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
