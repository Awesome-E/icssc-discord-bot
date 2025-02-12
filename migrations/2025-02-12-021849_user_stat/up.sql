-- Your SQL goes here
CREATE MATERIALIZED VIEW user_stat AS
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
