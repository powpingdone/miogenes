//! `SeaORM` Entity. Generated by sea-orm-codegen 0.10.3
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "playlist")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub owner: Uuid,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::Owner",
        to = "super::user::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    User,
    #[sea_orm(has_many = "super::join_playlist_track::Entity")]
    JoinPlaylistTrack,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::join_playlist_track::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JoinPlaylistTrack.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
