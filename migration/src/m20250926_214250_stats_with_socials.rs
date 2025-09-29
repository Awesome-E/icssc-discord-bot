use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP MATERIALIZED VIEW user_stat")
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                r#"CREATE MATERIALIZED VIEW user_stat AS SELECT u.id,
       COALESCE(socials_initiated.cnt, 0)::bigint AS socials_initiated,
       COALESCE(snipes_initiated.cnt, 0)::bigint  AS snipes_initiated,
       COALESCE(socials_victim.cnt, 0)::bigint    AS socials_victim,
       COALESCE(snipes_victim.cnt, 0)::bigint     AS snipes_victim
FROM (SELECT DISTINCT author_id AS id
      FROM message
      UNION
      SELECT DISTINCT victim_id
      FROM snipe) u
         LEFT JOIN (SELECT author_id, COUNT(*) AS cnt
                    FROM message
                             LEFT JOIN snipe ON message.message_id = snipe.message_id
                    WHERE message.is_social
                    GROUP BY author_id) socials_initiated ON u.id = socials_initiated.author_id
         LEFT JOIN (SELECT author_id, COUNT(*) AS cnt
                    FROM message
                             LEFT JOIN snipe ON message.message_id = snipe.message_id
                    WHERE NOT message.is_social
                    GROUP BY author_id) snipes_initiated ON u.id = snipes_initiated.author_id
         LEFT JOIN (SELECT victim_id, COUNT(*) AS cnt
                    FROM snipe
                             INNER JOIN message ON snipe.message_id = message.message_id
                    WHERE message.is_social
                    GROUP BY victim_id) socials_victim ON u.id = socials_victim.victim_id
         LEFT JOIN (SELECT victim_id, COUNT(*) AS cnt
                    FROM snipe
                             INNER JOIN message ON snipe.message_id = message.message_id
                    WHERE NOT message.is_social
                    GROUP BY victim_id) snipes_victim ON u.id = snipes_victim.victim_id;"#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP MATERIALIZED VIEW user_stat")
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                r#"
CREATE MATERIALIZED VIEW IF NOT EXISTS user_stat AS
SELECT u.id,
       COALESCE(snipe.cnt, 0)::bigint  AS snipe,
       COALESCE(sniped.cnt, 0)::bigint AS sniped,
       CASE
           WHEN COALESCE(sniped.cnt, 0) = 0 THEN NULL
           ELSE CAST(COALESCE(snipe.cnt, 0) AS DOUBLE PRECISION) / COALESCE(sniped.cnt, 0)
           END                         AS snipe_rate
FROM (SELECT DISTINCT author_id AS id
      FROM message
      UNION
      SELECT DISTINCT victim_id
      FROM snipe) u
         LEFT JOIN
     (SELECT author_id, COUNT(*) AS cnt
      FROM message
               LEFT JOIN snipe on message.message_id = snipe.message_id
      GROUP BY author_id) snipe
     ON u.id = snipe.author_id
         LEFT JOIN
         (SELECT victim_id, COUNT(*) AS cnt FROM snipe GROUP BY victim_id) sniped
         ON u.id = sniped.victim_id
ORDER BY snipe_rate DESC NULLS LAST;

REFRESH MATERIALIZED VIEW user_stat;
        "#,
            )
            .await?;

        Ok(())
    }
}
