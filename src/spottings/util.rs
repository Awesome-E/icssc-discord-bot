use anyhow::Context as _;
use entity::opt_out;
use itertools::Itertools as _;
use sea_orm::QueryFilter as _;
use sea_orm::{ColumnTrait as _, DatabaseConnection, EntityTrait as _};
use serenity::all::UserId;

pub async fn opted_out_among<Ids>(
    conn: &DatabaseConnection,
    ids: Ids,
) -> anyhow::Result<impl Iterator<Item = UserId>>
where
    Ids: Iterator<Item = UserId>,
{
    let got = opt_out::Entity::find()
        .filter(opt_out::Column::Id.is_in(ids.map(UserId::get).collect_vec()))
        .all(conn)
        .await
        .context("bulk opt out query")?;

    Ok(got
        .into_iter()
        .map(|opted_out| UserId::new(opted_out.id as u64)))
}
