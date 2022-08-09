use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // album art table
        manager
            .create_table(
                Table::create()
                    .table(AlbumArtTable::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AlbumArtTable::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(AlbumArtTable::BlurHash)
                            .blob(BlobSize::Medium)
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // artist table
        manager
            .create_table(
                Table::create()
                    .table(ArtistTable::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ArtistTable::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ArtistTable::Name).string().not_null())
                    .to_owned(),
            )
            .await?;

        // album table
        manager
            .create_table(Table::create()
            .table(AlbumTable::Table)
            .if_not_exists()
            .col(
                ColumnDef::new(AlbumTable::Id)
                    .uuid()
                    .not_null()
                    .primary_key(),
            )
            .col(ColumnDef::new(AlbumTable::Name).string().not_null())
            .to_owned()
        ).await?;
        
        // tracks table
        manager
            .create_table(
                Table::create()
                    .table(TrackTable::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TrackTable::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TrackTable::Title).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("AlbumArt_Link")
                            .from(TrackTable::Table, TrackTable::AlbumArtId)
                            .to(AlbumArtTable::Table, AlbumArtTable::Id),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AlbumArtTable::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(ArtistTable::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(TrackTable::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum AlbumArtTable {
    Table,
    Id,       // u128, UUID of ALBUM ART ON DISK
    BlurHash, // var len ASCII (binary), BLURHASH of ALBUM ART ON DISK
}

#[derive(Iden)]
enum ArtistTable {
    Table,
    Id,   // u128, UUID of ARTIST
    Name, // string, DISPLAY NAME of ARTIST
}

#[derive(Iden)]
enum AlbumTable {
    Table,
    Id, // u128, UUID of ALBUM
    Name, // string, DISPLAY NAME of ALBUM
    AlbumArtId, // u128, UUID of ALBUM ART
}

#[derive(Iden)]
enum TrackTable {
    Table,
    Id,         // u128, UUID of TRACK
    Title,      // string, Track Title (may be file name if no title is in metadata)
    ArtistId,   // Option u128, UUID of ARTIST
    AlbumId,    // Option u128, UUID of ALBUM
    AlbumArtId, // Option u128, UUID of ALBUMART
}
