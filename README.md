# ICSSC Discord Bot
Discord Utility Bot for ICSSC

## Features

### Spotting Logs

Discord bot to track "snipes", incidental encounters on the UCI campus, between members of
the [ICS Student Council](https://icssc.club). Massive thanks to the ICSSC Graphics Committee
for being awesome and drawing up custom assets in line with ICSSC and UCI's graphical themes.

### Matchy Meetups

Creates pairings between members for ICSSC's Matchy Meetups. Commands include 

## Local Development

### Setup

1. Clone the repo
2. `cargo install`
3. Set environment variables based on `.env.example`
4. `cargo run`

### Creating Database Migrations

- `sea-orm-cli migrate generate [name]`
- `sea-orm-cli migrate up`
- `sea-orm-cli generate entity -o entity/src/entities`
    - Revert the removed line in `entities/mod.rs` for the materialized view :P
