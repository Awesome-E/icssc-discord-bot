use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("snipe_message_id_fkey")
                    .table(Snipe::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .from(Snipe::Table, Snipe::MessageId)
                    .to(Message::Table, Message::MessageId)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("snipe_message_id_fkey")
                    .table(Snipe::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .from(Snipe::Table, Snipe::MessageId)
                    .to(Message::Table, Message::MessageId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Message {
    Table,
    GuildId,
    ChannelId,
    MessageId,
    AuthorId,
    TimePosted,
}

#[derive(DeriveIden)]
enum Snipe {
    Table,
    MessageId,
    VictimId,
    Latitude,
    Longitude,
    Notes,
}
