# Changelog

All notable changes to fs-sav are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html):
while it is 0.x, breaking changes to the [output contract](README.md#stability)
bump the minor version.

## [0.4.0] - 2026-10-13

### Added

- `refinery` field on the main stockpile of a refinery: the queue the player
  had selected, its orders (`source`, `refined`, `duration`) and the refined
  output held per owner and recipe.
- `factory_orders` field on the main stockpile of a Factory or Mass Production
  Factory: the player's order in each production queue, with its items and the
  seconds until it is done.
- `fs_sav.stockpile_key()` (Python) and `fs_sav::stockpile_key()` (Rust) to
  build a stockpile's key the same way fs-sav does.
- `find_save_files()` (Python and Rust) and the `fs-sav saves` command, which
  list the game's map save files on Windows, WSL and Linux (native and Flatpak
  Steam, including extra Steam libraries), newest first.
- Parse warnings: tooltips that yield no stockpiles are reported in `warnings`
  (Rust), as `UserWarning`s (Python) and on stderr (CLI).
- Documentation of the output format as a contract, including how empty and
  skipped stockpiles are handled.

### Changed

- **Breaking:** pinned tooltips whose latest details carry no data are skipped
  (with a warning) instead of being returned as empty stockpiles.
- **Breaking:** the stockpile key rounds coordinates to 4 decimals instead of
  6. Keys saved by earlier versions no longer match.
- **Breaking (Rust):** `parse_save_bytes` returns `ParsedStockpiles`
  (stockpiles and warnings) instead of `Vec<Stockpile>`, and `Stockpile` has
  new public fields.
- `tech` is only read from the tooltip's latest details, never from its
  initial snapshot.
- `parse_save` reads the whole file and retries while the game is writing it,
  so a save being written is never parsed. It fails with
  `FsSavError::FileUnstable` if the file never settles.
- Property names in the save are matched case-insensitively.

### Fixed

- `tech` for `RelicBase2`, `RelicBase3`, `GarrisonStation2` and
  `TownCLargeGarrisonGS1`, which share the tech trees of `RelicBase1` and
  `GarrisonStation`.
- Errors parsing a single tooltip are reported as warnings instead of only
  being printed to stderr.

## [0.3.0] - 2026-06-24

### Changed

- **Breaking:** the stockpile `type` is the raw in-game CodeName (a free-form
  string) instead of a fixed enum, so new structure types are kept instead of
  collapsing to `Undefined`.

## [0.2.5] - 2026-06-07

### Added

- `faction` filter in Python `parse_save` and `parse_save_bytes`.

### Fixed

- Reading the last updated timestamp of stockpiles.

## [0.2.4] - 2026-05-28

### Fixed

- Code formatting.

## [0.2.3] - 2026-05-28

### Added

- Garrison/upgrade `tech` of bases.

### Fixed

- Swapped Encampment/SafeHouse stockpile type mapping.

## [0.2.2] - 2026-05-26

### Added

- Faction filter flag.

### Fixed

- Parsing of Warden stockpiles.

## [0.2.1] - 2026-05-25

### Fixed

- Reading stockpile `quantity` from any integer property type.

## [0.2.0] - 2026-05-23

### Added

- `fs-sav` console script for pip installs.
- Standalone binaries attached to GitHub Releases.
- MIT license.

## [0.1.1] - 2026-05-23

### Fixed

- Python wheels use the stable ABI (abi3) and support Python 3.10 to 3.13.

## [0.1.0] - 2026-05-23

### Added

- First release: parse Foxhole `.sav` files and extract pinned stockpiles,
  with CLI, Rust library and Python bindings.

[0.4.0]: https://github.com/xurxogr/fs-sav/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/xurxogr/fs-sav/compare/v0.2.5...v0.3.0
[0.2.5]: https://github.com/xurxogr/fs-sav/compare/v0.2.4...v0.2.5
[0.2.4]: https://github.com/xurxogr/fs-sav/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/xurxogr/fs-sav/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/xurxogr/fs-sav/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/xurxogr/fs-sav/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/xurxogr/fs-sav/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/xurxogr/fs-sav/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/xurxogr/fs-sav/releases/tag/v0.1.0
