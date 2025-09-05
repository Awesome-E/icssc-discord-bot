#!/bin/bash
pg_dump \
--file=db.sql.gz \
--format=p \
--verbose \
--verbose \
--compress=gzip:9 \
--clean \
--no-owner \
--schema=public \
--no-privileges \
--if-exists \
--dbname=ics_spottings_council \
--host=localhost \
--port=5432 \
--user=postgres
