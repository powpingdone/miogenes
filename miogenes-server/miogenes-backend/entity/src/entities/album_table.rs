//! SeaORM Entity. Generated by sea-orm-codegen 0.9.1

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "album_table")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub ts: i64,
    pub name: String,
    pub album_art_id: Option<Uuid>,
    pub artist_id: Option<Uuid>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::album_art_table::Entity",
        from = "Column::AlbumArtId",
        to = "super::album_art_table::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    AlbumArtTable,
    #[sea_orm(
        belongs_to = "super::artist_table::Entity",
        from = "Column::ArtistId",
        to = "super::artist_table::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    ArtistTable,
    #[sea_orm(has_many = "super::track_table::Entity")]
    TrackTable,
}

impl Related<super::album_art_table::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AlbumArtTable.def()
    }
}

impl Related<super::artist_table::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ArtistTable.def()
    }
}

impl Related<super::track_table::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TrackTable.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
