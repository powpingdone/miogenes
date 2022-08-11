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
    Password, // 64 byte blob, blake2b sum of password
}

#[derive(Iden)]
enum TrackTable {
    Table,
    Owner,
}
