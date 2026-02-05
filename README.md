# ICSSC Discord Bot

![ICSSC Bot Banner](https://cdn.discordapp.com/banners/1413632282136154234/800e62e9f6339a923f2d52f398d646f9.png?size=1280)

A utility bot created for ICSSC's Discord server. *Massive thanks to the ICSSC Graphics Committee for being awesome and drawing up custom assets in line with ICSSC and UCI's graphical themes.*

## Features

### Attendance

Allows users to check in to ICSSC events with the `/checkin` command.

### Bits & Bytes

Allows board members to log bits & bytes meetups with context menu commands

### Matchy Meetups

Creates and sends pairings between members for ICSSC's Matchy Meetups every other week.
Pairings are created with a seed using `/matchy create` and DMd to all individuals using
`/matchy send`.

### Roster Syncing

Automatically checks which members are out of sync with the ICSSC roster when
`/roster check_discord_roles` or `/roster check_google_access` is run.

### Spotting Logs

Tracks "snipes" and "socials", incidental encounters on the UCI campus, between members of the [ICS Student Council](https://icssc.club).

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

## Todos
- make sure most commands use administrator
- make `Context`s less confusing between poise, serenity, and crate
  
