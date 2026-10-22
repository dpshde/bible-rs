# Contributing

## route.bible adapters

Do not clone route.bible or run npm to refresh Open in › adapters. Use `scripts/sync-adapters`
only (`ROUTE_BIBLE_FEED` overrides the base URL). The daily GitHub Action
`.github/workflows/sync-route-bible-adapters.yml` opens one PR per new feed version.
