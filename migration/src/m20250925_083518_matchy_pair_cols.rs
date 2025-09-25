use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(MatchyMeetupRound::Table)
                    .modify_column(timestamp(MatchyMeetupRound::CreatedAt).default(Expr::cust("NOW()")))
                    .to_owned(),
            )
            .await?;

        manager.drop_foreign_key(ForeignKey::drop()
            .name("matchy_meetup_pair_round_id_fkey")
            .table(MatchyMeetupPair::Table)
            .to_owned()).await?;

        manager.create_foreign_key(ForeignKey::create()
            .from(MatchyMeetupPair::Table, MatchyMeetupPair::RoundId)
            .to(MatchyMeetupRound::Table, MatchyMeetupRound::Id)
            .on_delete(ForeignKeyAction::Cascade).to_owned()).await?;

        manager.drop_foreign_key(ForeignKey::drop()
            .name("matchy_meetup_pair_member_pair_id_fkey")
            .table(MatchyMeetupPairMember::Table)
            .to_owned()).await?;

        manager.create_foreign_key(ForeignKey::create()
            .from(
                MatchyMeetupPairMember::Table,
                MatchyMeetupPairMember::PairId,
            )
            .to(MatchyMeetupPair::Table, MatchyMeetupPair::Id).on_delete(ForeignKeyAction::Cascade).to_owned()).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_foreign_key(ForeignKey::drop()
            .name("matchy_meetup_pair_member_pair_id_fkey")
            .table(MatchyMeetupPairMember::Table)
            .to_owned()).await?;

        manager.create_foreign_key(ForeignKey::create()
            .from(
                MatchyMeetupPairMember::Table,
                MatchyMeetupPairMember::PairId,
            )
            .to(MatchyMeetupPair::Table, MatchyMeetupPair::Id).to_owned()).await?;

        manager.drop_foreign_key(ForeignKey::drop()
            .name("matchy_meetup_pair_round_id_fkey")
            .table(MatchyMeetupPair::Table)
            .to_owned()).await?;

        manager.create_foreign_key(ForeignKey::create()
            .from(MatchyMeetupPair::Table, MatchyMeetupPair::RoundId)
            .to(MatchyMeetupRound::Table, MatchyMeetupRound::Id)
            .to_owned()).await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MatchyMeetupRound::Table)
                    .modify_column(timestamp(MatchyMeetupRound::CreatedAt))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum MatchyMeetupRound {
    Table,
    Id,
    CreatedAt,
}

#[derive(DeriveIden)]
enum MatchyMeetupPair {
    Table,
    RoundId,
    Id,
}

#[derive(DeriveIden)]
enum MatchyMeetupPairMember {
    Table,
    PairId,
}
