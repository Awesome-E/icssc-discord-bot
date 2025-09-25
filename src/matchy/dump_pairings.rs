use crate::{BotVars, Context};
use crate::matchy::discord_helpers::previous_matches;
use anyhow::{Error, Result};
use entity::{matchy_meetup_pair, matchy_meetup_pair_member, matchy_meetup_round};
use sea_orm::{ActiveModelTrait, DbErr, Set, TransactionTrait};
use crate::matchy::participation::get_previous_matches;

async fn handle_dump_pairings(ctx: &Context<'_>) -> Result<String> {
    let prev_matches = get_previous_matches(ctx.data()).await?;

    let round_sql = matchy_meetup_round::ActiveModel {
        id: Default::default(),
        created_at: Set(chrono::naive::NaiveDateTime::default()),
    };

    let conn = &ctx.data().db;
    let Ok(_) = conn
        .transaction::<_, (), DbErr>(move |txn| {
            Box::pin(async move {
                let round = round_sql.insert(txn).await.expect("insert round");

                for pair in &prev_matches {
                    let pair_sql = matchy_meetup_pair::ActiveModel {
                        id: Default::default(),
                        round_id: Set(round.id),
                    };
                    let pair_sql = pair_sql.insert(txn).await.expect("insert pair");

                    for userId in pair {
                        let pair_member_sql = matchy_meetup_pair_member::ActiveModel {
                            pair_id: Set(pair_sql.id),
                            discord_uid: Set((*userId).into()),
                        };
                        pair_member_sql
                            .insert(txn)
                            .await
                            .expect("insert pair member");
                    }
                }
                Ok(())
            })
        })
        .await
    else {
        return Ok(String::from("Error: unable to dump pairings into db"));
    };

    Ok(String::from("Dumped pairings to database"))
}

/// Dump pairing history from the current into the database
#[poise::command(slash_command, hide_in_help, required_permissions = "ADMINISTRATOR")]
pub async fn dump_pairings(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let resp = handle_dump_pairings(&ctx)
        .await
        .unwrap_or_else(|e| format!("Error: {e}"));
    println!("{resp}");
    ctx.say(resp).await?;
    Ok(())
}
