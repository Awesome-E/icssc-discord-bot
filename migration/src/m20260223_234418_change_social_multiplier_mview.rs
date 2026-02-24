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
                r#"
CREATE MATERIALIZED VIEW user_stat AS SELECT u.id,
       COALESCE(socials_initiated.cnt, 0)::bigint AS socials_initiated,
       COALESCE(snipes_initiated.cnt, 0)::bigint  AS snipes_initiated,
       COALESCE(socials_victim.cnt, 0)::bigint    AS socials_victim,
       COALESCE(snipes_victim.cnt, 0)::bigint     AS snipes_victim
FROM (SELECT DISTINCT author_id AS id
      FROM spotting_message
      UNION
      SELECT DISTINCT victim_id
      FROM spotting_victim) u
         LEFT JOIN (SELECT author_id, COUNT(*) AS cnt
                    FROM spotting_message msg
                    WHERE msg.is_social
                    GROUP BY author_id) socials_initiated ON u.id = socials_initiated.author_id
         LEFT JOIN (SELECT author_id, COUNT(*) AS cnt
                    FROM spotting_message msg
                             LEFT JOIN spotting_victim v ON msg.message_id = v.message_id
                    WHERE NOT msg.is_social
                    GROUP BY author_id) snipes_initiated ON u.id = snipes_initiated.author_id
         LEFT JOIN (SELECT victim_id, COUNT(*) AS cnt
                    FROM spotting_victim v
                             INNER JOIN spotting_message msg ON v.message_id = msg.message_id
                    WHERE msg.is_social
                    GROUP BY victim_id) socials_victim ON u.id = socials_victim.victim_id
         LEFT JOIN (SELECT victim_id, COUNT(*) AS cnt
                    FROM spotting_victim v
                             INNER JOIN spotting_message msg ON v.message_id = msg.message_id
                    WHERE NOT msg.is_social
                    GROUP BY victim_id) snipes_victim ON u.id = snipes_victim.victim_id;
                "#,
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared("REFRESH MATERIALIZED VIEW user_stat;")
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
CREATE MATERIALIZED VIEW user_stat AS SELECT u.id,
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
                    GROUP BY victim_id) snipes_victim ON u.id = snipes_victim.victim_id;"
        "#,
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared("REFRESH MATERIALIZED VIEW user_stat;")
            .await?;

        Ok(())
    }
}
