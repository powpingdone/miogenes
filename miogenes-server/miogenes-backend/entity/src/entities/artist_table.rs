//! SeaORM Entity. Generated by sea-orm-codegen 0.9.2

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "artist_table")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub ts: i64,
    pub name: String,
    pub owner: Uuid,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user_table::Entity",
        from = "Column::Owner",
        to = "super::user_table::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    UserTable,
    #[sea_orm(has_many = "super::track_table::Entity")]
    TrackTable,
    #[sea_orm(has_many = "super::album_table::Entity")]
    AlbumTable,
}

impl Related<super::user_table::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserTable.def()
    }
}

impl Related<super::track_table::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TrackTable.def()
    }
}

impl Related<super::album_table::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AlbumTable.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
