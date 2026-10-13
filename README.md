# fs-sav

Foxhole save file parser - extracts stockpile data from `.sav` files.

## Features

- Parse Foxhole save files (`.sav`) using the [uesave](https://github.com/trumank/uesave) library
- Extract stockpile information: items, locations, types, quantities
- Find the game's save files (Windows, WSL, Steam Proton and Flatpak on Linux)
- Watch files for changes with diff modes (NDJSON output)
- Filter by hex, stockpile type, public/reserve status
- Output compatible with [foxhole-stockpiles](https://github.com/xurxogr/foxhole-stockpiles)
- Python bindings via PyO3

## Installation

### Python (from PyPI)

```bash
pip install fs-sav
```

### Rust CLI (from source)

```bash
cargo install --path .
```

### Python (from source)

```bash
pip install maturin
maturin develop --features python
```

## CLI Usage

### Parse command

```bash
# Parse a save file
fs-sav parse path/to/War.sav

# Parse with compact output (no pretty printing)
fs-sav parse path/to/War.sav --compact

# Read from stdin
cat War.sav | fs-sav parse
```

### Filter options

```bash
# Only public stockpiles (non-reserve)
fs-sav parse War.sav --public

# Only reserve stockpiles
fs-sav parse War.sav --reserves

# Filter by hex
fs-sav parse War.sav --hex TerminusHex

# Filter by stockpile type
fs-sav parse War.sav --type Seaport

# Only stockpiles with items
fs-sav parse War.sav --with-items

# Combine filters
fs-sav parse War.sav --hex TerminusHex --public --with-items
```

### Watch command

```bash
# Watch for changes (outputs all stockpiles on each change)
fs-sav watch path/to/War.sav

# Watch with custom poll interval (seconds)
fs-sav watch path/to/War.sav --poll 2.0

# Only output stockpiles that changed (any field)
fs-sav watch path/to/War.sav --diff

# Only output stockpiles where items changed
fs-sav watch path/to/War.sav --diff-items

# Watch with filters
fs-sav watch path/to/War.sav --hex TerminusHex --public
```

### Saves command

```bash
# List the map save files found on this machine (JSON, newest first)
fs-sav saves
```

```json
[
  {
    "path": "C:\\Users\\me\\AppData\\Local\\Foxhole\\Saved\\SaveGames\\76561197960293553_MapData.sav",
    "steam_id": "76561197960293553",
    "modified": "2026-08-01T16:25:36Z",
    "size_bytes": 143466
  }
]
```

The game keeps one `<SteamID64>_MapData.sav` per Steam account. They are
looked for in:

- Windows: `%LOCALAPPDATA%\Foxhole\Saved\SaveGames`
- WSL: the same folder of every Windows user, under `/mnt/c/Users`
- Linux: the Proton prefix (`steamapps/compatdata/505460/...`) of every Steam
  library, for native Steam (`~/.steam/steam`, `~/.steam/root`,
  `~/.steam/debian-installation`, `~/.local/share/Steam`) and Flatpak Steam
  (`~/.var/app/com.valvesoftware.Steam/.local/share/Steam`), including the
  extra libraries listed in `libraryfolders.vdf`

A save elsewhere is read by passing its path to `parse` or `watch`.

### Version

```bash
fs-sav version
```

## Python Usage

```python
import fs_sav

# Parse a save file
stockpiles = fs_sav.parse_save("path/to/War.sav")
print(f"Found {len(stockpiles)} stockpiles")

for stockpile in stockpiles:
    print(f"  {stockpile['name']} ({stockpile['type']}): {len(stockpile['items'])} items")

# Parse with filters
seaports = fs_sav.parse_save("War.sav", stockpile_type="Seaport")
reserves = fs_sav.parse_save("War.sav", reserves=True)
terminus_public = fs_sav.parse_save("War.sav", hex="TerminusHex", public=True)

# Parse from bytes
with open("War.sav", "rb") as f:
    data = f.read()
stockpiles = fs_sav.parse_save_bytes(data)

# Track stockpiles across parses by their key
by_key = {fs_sav.stockpile_key(s): s for s in stockpiles}

# Find the save files on this machine (newest first)
saves = fs_sav.find_save_files()
if saves:
    stockpiles = fs_sav.parse_save(saves[0]["path"])

# Get parser info
info = fs_sav.info()
print(f"Implementation: {info['implementation']}, Version: {info['version']}")
```

Pinned tooltips that yield no stockpiles (see [Skipped tooltips](#skipped-tooltips))
are reported as `UserWarning`s:

```python
import warnings

with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always")
    stockpiles = fs_sav.parse_save("War.sav")
skipped = [str(w.message) for w in caught]
```

### Filter parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `public` | `bool` | Only public stockpiles (non-reserve) |
| `reserves` | `bool` | Only reserve stockpiles |
| `hex` | `str` | Filter by hex name (e.g., "TerminusHex") |
| `stockpile_type` | `str` | Filter by type (e.g., "Seaport") |
| `with_items` | `bool` | Only stockpiles with items (drops empty and newly pinned ones) |

## Rust Library Usage

```rust
use fs_sav::{parse_save, ParseResult};

fn main() -> fs_sav::Result<()> {
    let result = parse_save("path/to/War.sav")?;
    println!("Found {} stockpiles", result.stockpiles.len());

    for stockpile in &result.stockpiles {
        println!("  {} ({:?}): {} items",
            stockpile.name,
            stockpile.stockpile_type,
            stockpile.items.len()
        );
    }
    for warning in &result.warnings {
        eprintln!("skipped: {warning}");
    }

    // From bytes: a `ParsedStockpiles` with the same stockpiles and warnings
    let parsed = fs_sav::parse_save_bytes(&std::fs::read("path/to/War.sav")?)?;
    println!("Found {} stockpiles", parsed.stockpiles.len());

    // Find the save files on this machine (newest first)
    if let Some(save) = fs_sav::find_save_files().first() {
        let result = parse_save(&save.path)?;
        println!("{}: {} stockpiles", save.path.display(), result.stockpiles.len());
    }

    Ok(())
}
```

## Output Format

The output is a JSON array of stockpiles (Python: a list of dictionaries):

```json
[
  {
    "name": "",
    "type": "Seaport",
    "faction": "Colonial",
    "hex": "TerminusHex",
    "coords": { "x": 0.457745, "y": 0.664469 },
    "is_reserve": false,
    "items": [
      { "code": "Rifle", "quantity": 100, "crated": false },
      { "code": "RifleAmmo", "quantity": 50, "crated": true }
    ],
    "timestamp": "2024-01-15T10:29:00Z"
  }
]
```

### Stockpile fields

| Field | Type | Presence | Description |
|-------|------|----------|-------------|
| `name` | `str` | always | `""` for a structure's main stockpile, the reserve's name for a reserve stockpile |
| `type` | `str` | always | Raw in-game CodeName (e.g. `Seaport`). Unknown or new types are passed through as is |
| `faction` | `str` | always | `Warden` or `Colonial`: the faction whose pinned tooltips the stockpile was read from |
| `hex` | `str` | when known | Hex name, e.g. `TerminusHex` |
| `coords` | `{x, y}` | when known | Normalized map coordinates (0.0 to 1.0) |
| `is_reserve` | `bool` | always | Whether this is a reserve stockpile |
| `items` | list | always | Items in the stockpile; empty for an empty stockpile (see below) |
| `items[].code` | `str` | always | Item CodeName |
| `items[].quantity` | `int` | always | Quantity |
| `items[].crated` | `bool` | always | Whether the item is crated |
| `tech` | object | bases only | Garrison/upgrade build progress (0-100 per tech component) |
| `refinery` | object | refineries only | See [Refinery Data](#refinery-data) |
| `factory_orders` | list | factories only | See [Factory Orders](#factory-orders) |
| `timestamp` | `str` or `null` | always | When the tooltip was last updated (UTC, ISO 8601); `null` if missing in the save, with an entry in `errors` |
| `errors` | list of `str` | when any | Problems found reading the stockpile |

Fields that are not present are omitted rather than set to `null` (except
`timestamp`). Items never carry `confidence`, and `shard`, `ingame_timestamp`
and `resolution` are never set: those only exist in screenshot (OCR) output.

### Empty stockpiles

Empty stockpiles are returned like any other, with `items: []`. This includes
newly pinned stockpiles, which start empty. The `with_items` filter drops them,
so leave it off when tracking stockpiles over time.

### Skipped tooltips

Only the latest details of each pinned tooltip are read. A tooltip whose
latest details carry no data is skipped: the game sends none for structures
the player is not on the side of, and stores empty details when it has none
for the structure (e.g. it no longer exists, or the tooltip could not be
matched to it). Skipped (and unreadable) tooltips are reported as warnings: in
`warnings` of the Rust results, as Python `UserWarning`s, and on stderr by the
CLI.

### Stockpile key

Each stockpile has a key that identifies it across parses, used by
`fs-sav watch` to track changes:

```
type:hex:x,y:name
```

Coordinates are rounded to 4 decimals (`0,0` when absent), an empty type is
written as `Undefined` and a missing hex as an empty string, e.g.
`Seaport:TerminusHex:0.4577,0.6645:`. Build it with `fs_sav.stockpile_key(stockpile)`
in Python or `Stockpile::to_key()` / `fs_sav::stockpile_key()` in Rust rather
than reimplementing it, so it stays consistent across versions.

### Stability

The fields above, the key format and the input of `parse_save_bytes` (the raw
bytes of a `.sav` file) are the public contract. New optional fields may be
added in any release, so ignore fields you don't use. Removing or renaming a
field, changing its type or meaning, or changing the key format is a breaking
change: it bumps the minor version while fs-sav is 0.x, and the major version
from 1.0.

Changes between versions, breaking ones first, are listed in the
[CHANGELOG](CHANGELOG.md).

## Refinery Data

The main stockpile of a refinery also carries a `refinery` object with its
production state, as seen by the player who last opened the tooltip:

```json
{
  "type": "Refinery",
  "items": [{ "code": "Diesel", "quantity": 2340, "crated": false }],
  "refinery": {
    "orders_access": { "level": "public" },
    "orders": [
      { "slot": 0, "code": "Cloth", "source": 0, "refined": 50, "duration": 0 }
    ],
    "storages": [
      {
        "access": { "level": "squad", "squad_id": 1 },
        "slot": 1,
        "code": "Diesel",
        "refined": 5309
      }
    ]
  }
}
```

- `orders_access`: the queue the player had selected (`level` is `personal`,
  `public` or `squad`; `squad_id` only for squad queues).
- `orders`: one entry per recipe of that queue. `source` is the material still
  waiting to be refined, `refined` the output ready to collect (for a squad
  queue, the squad's stored output) and `duration` the seconds left to refine
  the player's remaining source.
- `storages`: output held per owner and recipe: the player's other squads and,
  while viewing a squad queue, the player's own personal output.
- `slot` is the recipe's position in the refinery's recipe list (255 if not
  in the list); `code` is the item it produces, when known.
- Counts are capped by the game at 65000 and `duration` at 64800 seconds.

## Factory Orders

The main stockpile of a Factory or Mass Production Factory also carries the
orders of the player who last opened the tooltip, one per production queue:

```json
{
  "type": "Factory",
  "factory_orders": [
    {
      "queue": 0,
      "queue_type": "SmallArms",
      "duration": 0,
      "items": [{ "slot": 16, "code": "RifleC" }]
    },
    {
      "queue": 6,
      "queue_type": "Uniforms",
      "duration": 90,
      "items": [{ "slot": 13, "code": "ScoutUniformC" }]
    }
  ]
}
```

- `queue` is the queue's position in the factory's queue list; `queue_type`
  names it when known. A Mass Production Factory's vehicle and structure
  queues are not reported in tooltips.
- `duration` is the seconds until the order is done, counting the orders ahead
  of it in the queue (capped by the game at 64800); it is 0 for an order that
  is done and waiting to be collected, which is reported before pending ones.
- `items` are the order's items: `slot` is the position in the queue's
  production list (255 if not in the list) and `code` the item, when known.

## Stockpile Types

`type` is the structure's in-game CodeName, passed through as is, so any
structure that can be pinned is supported, including ones added by game
updates. Some common types:

| Category | Types |
|----------|-------|
| **Bases** | GarrisonStation, Keep, ForwardBase1, RelicBase (1-3), FortBase (T1-T3), BorderBase, TownBase (1-3), FortGarrisonStation |
| **Storage** | StorageFacility, Seaport, AircraftDepot, ConstructionYard |
| **Production** | Refinery, Factory, MassProduction |
| **Facilities** | Hospital, MaintenanceTunnel, FacilityFactorySmallArms, FacilityModificationCenter, FacilityTransfer (Liquid/Material/Resource), FacilityVehicleFactory (1-3) |

## License

MIT
