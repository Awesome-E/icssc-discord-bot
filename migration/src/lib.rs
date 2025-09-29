pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_table;
mod m20250905_213900_matchy_history;
mod m20250916_174534_social_spottings;
mod m20250923_231905_matchy_opt_in;
mod m20250925_083518_matchy_pair_cols;
mod m20250926_214250_stats_with_socials;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20250905_213900_matchy_history::Migration),
            Box::new(m20250916_174534_social_spottings::Migration),
            Box::new(m20250923_231905_matchy_opt_in::Migration),
            Box::new(m20250925_083518_matchy_pair_cols::Migration),
            Box::new(m20250926_214250_stats_with_socials::Migration),
        ]
    }
}
