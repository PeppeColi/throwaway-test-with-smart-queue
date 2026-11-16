# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
The agent adds one line under "Unreleased" in each pull request; when you release, move those
lines under a new heading with the version and the date.

## [Unreleased]

### Added

- `ricette recipes list` and `ricette recipes search <ingredient>` show the built-in recipe catalog and find the recipes that use an ingredient.
- `ricette list add`, `ricette list show` and `ricette list clear` manage a shopping list stored in `shopping-list.json`.
- `ricette list add <item> --qty <n> --unit <u>` adds a measured entry; an entry with the same item and unit adds its quantity to the existing line.
