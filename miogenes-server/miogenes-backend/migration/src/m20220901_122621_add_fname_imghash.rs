use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(TrackTable::Table)
                    .add_column(ColumnDef::new(TrackTable::OrigFname).string().not_null())
                    .add_column(ColumnDef::new(TrackTable::ExtraTags).string().not_null())
                    .add_column(ColumnDef::new(TrackTable::Hash).binary_len(32).not_null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(AlbumArtTable::Table)
                    .add_column(
                        ColumnDef::new(AlbumArtTable::Hash)
                            .binary_len(32)
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(AlbumArtTable::Table)
                    .drop_column(AlbumArtTable::Hash)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(TrackTable::Table)
                    .drop_column(TrackTable::Hash)
                    .drop_column(TrackTable::OrigFname)
                    .drop_column(TrackTable::ExtraTags)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum TrackTable {
    Table,
    OrigFname, // string, sanitized filename
    Hash,      // blob, sha256 hash
    ExtraTags, // string, extra tags
}

#[derive(Iden)]
enum AlbumArtTable {
    Table,
    Hash, // blob, sha256 hash
}
