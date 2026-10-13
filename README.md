# fs-sav

Foxhole save file parser - extracts stockpile data from `.sav` files.

## Features

- Parse Foxhole save files (`.sav`) using the [uesave](https://github.com/trumank/uesave) library
- Extract stockpile information: items, locations, types, quantities
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

# Get parser info
info = fs_sav.info()
print(f"Implementation: {info['implementation']}, Version: {info['version']}")
```

### Filter parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `public` | `bool` | Only public stockpiles (non-reserve) |
| `reserves` | `bool` | Only reserve stockpiles |
| `hex` | `str` | Filter by hex name (e.g., "TerminusHex") |
| `stockpile_type` | `str` | Filter by type (e.g., "Seaport") |
| `with_items` | `bool` | Only stockpiles with items |

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

    Ok(())
}
```

## Output Format

The output is a JSON array of stockpiles:

```json
[
  {
    "name": "",
    "type": "Seaport",
    "hex": "TerminusHex",
    "coords": { "x": 0.457, "y": 0.664 },
    "is_reserve": false,
    "items": [
      { "code": "Rifle", "quantity": 100, "crated": false },
      { "code": "RifleAmmo", "quantity": 50, "crated": true }
    ],
    "timestamp": "2024-01-15T10:29:00Z"
  }
]
```

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

## Stockpile Types

The parser recognizes all Foxhole stockpile types:

| Category | Types |
|----------|-------|
| **Bases** | GarrisonStation, Keep, ForwardBase1, RelicBase1, FortBase (T1-T3), BorderBase, TownBase (T1-T3), FortGarrisonStation |
| **Storage** | StorageFacility, Seaport, AircraftDepot |
| **Facilities** | Hospital, Refinery, MaintenanceTunnel, FacilityFactorySmallArms, FacilityModificationCenter, FacilityTransfer (Liquid/Material/Resource), FacilityVehicleFactory (T1-T3) |

## License

MIT
