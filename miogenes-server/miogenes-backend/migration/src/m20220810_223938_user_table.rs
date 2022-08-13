use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserTable::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserTable::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(UserTable::Username).string().not_null())
                    .col(
                        ColumnDef::new(UserTable::Password)
                            .binary_len(32)
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;
        
        manager
            .alter_table(
                Table::alter()
                    .table(AlbumArtTable::Table)
                    .add_column(ColumnDef::new(AlbumArtTable::Owner).uuid().not_null())
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("AlbumArtToOwner_Link")
                            .from_tbl(AlbumArtTable::Table)
                            .from_col(AlbumArtTable::Owner)
                            .to_tbl(UserTable::Table)
                            .to_col(UserTable::Id),
                    )
                    .to_owned(),
            )
            .await?;
        
        manager
            .alter_table(
                Table::alter()
                    .table(AlbumTable::Table)
                    .add_column(ColumnDef::new(AlbumTable::Owner).uuid().not_null())
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("AlbumToOwner_Link")
                            .from_tbl(AlbumTable::Table)
                            .from_col(AlbumTable::Owner)
                            .to_tbl(UserTable::Table)
                            .to_col(UserTable::Id),
                    )
                    .to_owned(),
            )
            .await?;
        
        manager
            .alter_table(
                Table::alter()
                    .table(ArtistTable::Table)
                    .add_column(ColumnDef::new(ArtistTable::Owner).uuid().not_null())
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("ArtistToOwner_Link")
                            .from_tbl(ArtistTable::Table)
                            .from_col(ArtistTable::Owner)
                            .to_tbl(UserTable::Table)
                            .to_col(UserTable::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(TrackTable::Table)
                    .add_column(ColumnDef::new(TrackTable::Owner).uuid().not_null())
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("TrackToOwner_Link")
                            .from_tbl(TrackTable::Table)
                            .from_col(TrackTable::Owner)
                            .to_tbl(UserTable::Table)
                            .to_col(UserTable::Id),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(TrackTable::Table)
                    .drop_foreign_key(Alias::new("TrackToOwner_Link"))
                    .drop_column(TrackTable::Owner)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(ArtistTable::Table)
                    .drop_foreign_key(Alias::new("ArtistToOwner_Link"))
                    .drop_column(ArtistTable::Owner)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(AlbumTable::Table)
                    .drop_foreign_key(Alias::new("AlbumToOwner_Link"))
                    .drop_column(AlbumTable::Owner)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(AlbumArtTable::Table)
                    .drop_foreign_key(Alias::new("AlbumArtToOwner_Link"))
                    .drop_column(AlbumArtTable::Owner)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(UserTable::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum UserTable {
    Table,
    Id,       // u128, UUID for users
    Username, // string, username
    Password, // 32 byte blob, sha256 sum of password
}

#[derive(Iden)]
enum TrackTable {
    Table,
    Owner,
}

#[derive(Iden)]
enum AlbumArtTable {
    Table,
    Owner,
}

#[derive(Iden)]
enum ArtistTable {
    Table,
    Owner,
}

#[derive(Iden)]
enum AlbumTable {
    Table,
    Owner,
}