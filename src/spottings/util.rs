use anyhow::Context as _;
use entity::opt_out;
use itertools::Itertools as _;
use sea_orm::QueryFilter as _;
use sea_orm::{ColumnTrait as _, DatabaseConnection, EntityTrait as _};
use serenity::all::UserId;

pub async fn opted_out_among(
    conn: &DatabaseConnection,
    ids: impl Iterator<Item = UserId>,
) -> anyhow::Result<impl Iterator<Item = UserId>> {
    let got = opt_out::Entity::find()
        .filter(opt_out::Column::Id.is_in(ids.map(UserId::get).collect_vec()))
        .all(conn)
        .await
        .context("bulk opt out query")?;

    Ok(got
        .into_iter()
        .map(|opted_out| UserId::new(opted_out.id as u64)))
}
