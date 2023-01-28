use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // album
        manager
            .create_table(
                Table::create()
                    .table(Album::Table)
                    .if_not_exists()
                    // ser
                    .col(ColumnDef::new(Album::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Album::Title).string().not_null())
                    .col(ColumnDef::new(Album::SortName).string().null())
                    .to_owned(),
            )
            .await?;

        // coverart
        manager.create_table(Table::create()
            .table(CoverArt::Table)
            .if_not_exists()
            // ser
            .col(ColumnDef::new(CoverArt::Id).uuid().not_null().primary_key())
            .col(ColumnDef::new(CoverArt::WebmBlob).blob(BlobSize::Long).not_null())
            // noser
            .col(ColumnDef::new(CoverArt::ImgHash).binary_len(32).not_null())
            .to_owned()).await?;

        // artist
        manager
            .create_table(
                Table::create()
                    .table(Artist::Table)
                    .if_not_exists()
                    // ser
                    .col(ColumnDef::new(Artist::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Artist::Name).string().not_null())
                    .col(ColumnDef::new(Artist::SortName).string().null())
                    .to_owned(),
            )
            .await?;

        // playlist
        manager
            .create_table(
                Table::create()
                    .table(Playlist::Table)
                    .if_not_exists()
                    // ser
                    .col(ColumnDef::new(Playlist::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Playlist::Name).string().not_null())
                    // fk
                    .col(ColumnDef::new(Playlist::Owner).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Playlist::Table, Playlist::Owner)
                            .to(User::Table, User::Id)
                            .on_update(ForeignKeyAction::NoAction)
                            .on_delete(ForeignKeyAction::NoAction),
                    )
                    .to_owned(),
            )
            .await?;

        // user
        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .if_not_exists()
                    // ser
                    .col(ColumnDef::new(User::Id).uuid().not_null().primary_key())
                    // noser
                    .col(ColumnDef::new(User::Username).string().not_null())
                    .col(ColumnDef::new(User::Password).string().not_null())
                    .to_owned(),
            )
            .await?;

        // usertoken
        manager
            .create_table(
                Table::create()
                    .table(UserToken::Table)
                    .if_not_exists()
                    // ser
                    .col(ColumnDef::new(UserToken::Id).uuid().not_null().primary_key())
                    // noser
                    .col(ColumnDef::new(UserToken::Expiry).date_time().not_null())
                    // fk
                    .col(ColumnDef::new(UserToken::UserId).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(UserToken::Table, UserToken::UserId)
                            .to(User::Table, User::Id)
                            .on_update(ForeignKeyAction::NoAction)
                            .on_delete(ForeignKeyAction::NoAction),
                    )
                    .to_owned(),
            )
            .await?;

        // track
        manager
            .create_table(
                Table::create()
                    .table(Track::Table)
                    .if_not_exists()
                    // ser
                    .col(ColumnDef::new(Track::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Track::Title).string().not_null())
                    .col(ColumnDef::new(Track::SortName).string().null())
                    .col(ColumnDef::new(Track::Tags).json().not_null())
                    // noser
                    .col(ColumnDef::new(Track::AudioHash).binary_len(32).not_null())
                    .col(ColumnDef::new(Track::OrigFname).string().not_null())
                    // fk
                    .col(ColumnDef::new(Track::Album).uuid().null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Track::Table, Track::Album)
                            .to(Album::Table, Album::Id)
                            .on_update(ForeignKeyAction::NoAction)
                            .on_delete(ForeignKeyAction::NoAction),
                    )
                    .col(ColumnDef::new(Track::Artist).uuid().null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Track::Table, Track::Artist)
                            .to(Artist::Table, Artist::Id)
                            .on_update(ForeignKeyAction::NoAction)
                            .on_delete(ForeignKeyAction::NoAction),
                    )
                    .col(ColumnDef::new(Track::CoverArt).uuid().null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Track::Table, Track::CoverArt)
                            .to(CoverArt::Table, CoverArt::Id)
                            .on_update(ForeignKeyAction::NoAction)
                            .on_delete(ForeignKeyAction::NoAction),
                    )
                    .col(ColumnDef::new(Track::Owner).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Track::Table, Track::Owner)
                            .to(User::Table, User::Id)
                            .on_update(ForeignKeyAction::NoAction)
                            .on_delete(ForeignKeyAction::NoAction),
                    )
                    .to_owned(),
            )
            .await?;

        // joinplaylisttrack
        manager
            .create_table(
                Table::create()
                    .table(JoinPlaylistTrack::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(JoinPlaylistTrack::Id).big_integer().not_null().auto_increment().primary_key(),
                    )
                    // fk
                    .col(ColumnDef::new(JoinPlaylistTrack::PlaylistId).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(JoinPlaylistTrack::Table, JoinPlaylistTrack::PlaylistId)
                            .to(Playlist::Table, Playlist::Id)
                            .on_update(ForeignKeyAction::NoAction)
                            .on_delete(ForeignKeyAction::NoAction),
                    )
                    .col(ColumnDef::new(JoinPlaylistTrack::TrackId).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(JoinPlaylistTrack::Table, JoinPlaylistTrack::TrackId)
                            .to(Track::Table, Track::Id)
                            .on_update(ForeignKeyAction::NoAction)
                            .on_delete(ForeignKeyAction::NoAction),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(JoinPlaylistTrack::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Track::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(UserToken::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(User::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Playlist::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Album::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(CoverArt::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Artist::Table).to_owned()).await
    }
}

#[derive(Iden)]
enum User {
    Table,
    // ser
    Id,
    // noser
    Username,
    Password,
}

#[derive(Iden)]
enum UserToken {
    Table,
    // ser
    Id,
    // noser
    Expiry,
    // fk
    UserId,
}

#[derive(Iden)]
enum Album {
    Table,
    // ser
    Id,
    Title,
    SortName,
}

#[derive(Iden)]
enum Artist {
    Table,
    // ser
    Id,
    Name,
    SortName,
}

#[derive(Iden)]
enum Playlist {
    Table,
    // ser
    Id,
    Name,
    // fk
    Owner,
}

#[derive(Iden)]
enum JoinPlaylistTrack {
    Table,
    Id,
    // fk
    PlaylistId,
    TrackId,
}

#[derive(Iden)]
enum CoverArt {
    Table,
    // ser
    Id,
    WebmBlob,
    // noser
    ImgHash,
}

#[derive(Iden)]
enum Track {
    Table,
    // ser
    Id,
    Title,
    Tags,
    SortName,
    // noser
    AudioHash,
    OrigFname,
    // fk
    Album,
    CoverArt,
    Artist,
    Owner,
}
