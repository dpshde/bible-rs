# Contributing

## route.bible adapters

Open in › adapters are vendored under `crates/core/data/`. Default builds stay offline. To
refresh from the route.bible feed, run `scripts/sync-adapters` (or
`ROUTE_BIBLE_ADAPTERS=refresh cargo build -p bible-core`); override the base URL with
`ROUTE_BIBLE_FEED`. Do not clone route.bible or use npm for this.
