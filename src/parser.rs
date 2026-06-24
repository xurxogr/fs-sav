//! SAV file parser using uesave library.

use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::Path;

use chrono::{DateTime, TimeZone, Utc};
use uesave::{ByteArray, Properties, Property, Save, SaveReader, StructValue, ValueVec};

use crate::error::{FsSavError, Result};
use crate::models::{parse_tech, Faction, ParseResult, Stockpile, StockpileCoords, StockpileItem};

/// Parse a .sav file and extract stockpiles.
pub fn parse_save<P: AsRef<Path>>(path: P) -> Result<ParseResult> {
    let path = path.as_ref();

    if !path.exists() {
        return Err(FsSavError::FileNotFound(path.display().to_string()));
    }

    // Get file modification time
    let metadata = std::fs::metadata(path)?;
    let modified = metadata
        .modified()
        .map(DateTime::<Utc>::from)
        .unwrap_or_else(|_| Utc::now());

    // Read and parse the save file
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let save = SaveReader::new()
        .error_to_raw(true)
        .read(&mut reader)
        .map_err(|e| FsSavError::ParseError(e.to_string()))?;

    // Extract stockpiles from the save
    let stockpiles = extract_stockpiles(&save)?;

    Ok(ParseResult {
        parsed_at: Utc::now(),
        save_file: path.display().to_string(),
        save_file_modified: modified,
        stockpiles,
        warnings: vec![],
    })
}

/// Parse a .sav file from bytes.
pub fn parse_save_bytes(data: &[u8]) -> Result<Vec<Stockpile>> {
    let mut cursor = Cursor::new(data);

    let save = SaveReader::new()
        .error_to_raw(true)
        .read(&mut cursor)
        .map_err(|e| FsSavError::ParseError(e.to_string()))?;

    extract_stockpiles(&save)
}

/// Extract stockpiles from parsed save data.
fn extract_stockpiles(save: &Save) -> Result<Vec<Stockpile>> {
    let mut stockpiles = Vec::new();

    // Navigate to root.properties
    let properties = &save.root.properties;

    // Pinned tooltips are stored per-faction:
    //   PinnedMapToolTipsC -> Colonial
    //   PinnedMapToolTipsW -> Warden
    // A save may contain either or both.
    for (prop_name, faction) in [
        ("PinnedMapToolTipsC", Faction::Colonial),
        ("PinnedMapToolTipsW", Faction::Warden),
    ] {
        let pinned_tooltips = properties.0.iter().find_map(|(key, prop)| {
            if key.1 == prop_name {
                if let Property::Array(ValueVec::Struct(arr)) = prop {
                    return Some(arr);
                }
            }
            None
        });

        let Some(tooltips) = pinned_tooltips else {
            continue; // This faction's tooltips not present
        };

        // Parse each tooltip
        for tooltip in tooltips {
            if let StructValue::Struct(props) = tooltip {
                match parse_tooltip(props, faction) {
                    Ok(mut parsed) => stockpiles.append(&mut parsed),
                    Err(e) => {
                        // Log warning but continue
                        eprintln!("Warning: Failed to parse tooltip: {}", e);
                    }
                }
            }
        }
    }

    Ok(stockpiles)
}

/// Parse a single tooltip into stockpiles (main + reserves).
fn parse_tooltip(props: &Properties, faction: Faction) -> Result<Vec<Stockpile>> {
    let mut result = Vec::new();

    // Extract common fields
    let code_name = get_string_prop(props, "CodeName").unwrap_or_default();
    // Keep the raw in-game CodeName as the type. Unknown/new types stay valid
    // instead of collapsing to "Undefined"; only a missing CodeName falls back.
    let stockpile_type = if code_name.is_empty() {
        "Undefined".to_string()
    } else {
        code_name.clone()
    };

    // Extract map/region info
    let map_id = get_string_prop(props, "MapId").map(|s| {
        // Clean up: "EWorldConquestMapId::TerminusHex" -> "TerminusHex"
        s.split("::").last().unwrap_or(&s).to_string()
    });

    // Extract coordinates
    let coords = get_struct_prop(props, "NormalizedMapCoords").and_then(|sv| match sv {
        StructValue::Vector2D(v) => Some(StockpileCoords { x: v.x.0, y: v.y.0 }),
        StructValue::Struct(coord_props) => {
            let x = get_float_prop(coord_props, "x").unwrap_or(0.0);
            let y = get_float_prop(coord_props, "y").unwrap_or(0.0);
            Some(StockpileCoords { x, y })
        }
        _ => None,
    });

    // Get timestamp. `LastUpdated` is a DateTime struct (UE ticks as u64),
    // not a plain Int64 property. A missing/invalid value yields None (and an
    // error) instead of a fabricated "now" that would mask the failure.
    let timestamp = get_datetime_ticks_prop(props, "LastUpdated").and_then(parse_ue_timestamp);
    let timestamp_error = timestamp
        .is_none()
        .then(|| vec!["missing or invalid LastUpdated timestamp".to_string()]);

    // Get stockpile details from RecentMapItemDetails
    let details = get_struct_prop(props, "RecentMapItemDetails");

    if let Some(StructValue::Struct(detail_props)) = details {
        // Parse main stockpile items
        let stockpile_info = get_struct_prop(detail_props, "StockpileInfo");
        let items = stockpile_info
            .map(parse_stockpile_items)
            .unwrap_or_default();

        // Garrison/upgrade tech build progress (bases only). Prefer the recent
        // snapshot, but fall back to the initial details since the recent
        // Values array is often empty for tooltips not recently expanded.
        let tech = get_byte_array_prop(detail_props, "Values")
            .filter(|values| !values.is_empty())
            .or_else(|| {
                get_struct_prop(props, "InitalMapItemDetails").and_then(|sv| match sv {
                    StructValue::Struct(inital_props) => {
                        get_byte_array_prop(inital_props, "Values")
                    }
                    _ => None,
                })
            })
            .and_then(|values| parse_tech(&stockpile_type, values));

        // Main stockpile (public)
        result.push(Stockpile {
            name: String::new(),
            stockpile_type: stockpile_type.clone(),
            faction,
            hex: map_id.clone(),
            coords: coords.clone(),
            is_reserve: false,
            items,
            tech,
            timestamp,
            shard: None,
            ingame_timestamp: None,
            resolution: None,
            errors: timestamp_error.clone(),
        });

        // Parse reserve stockpiles
        if let Some(ValueVec::Struct(reserve_structs)) =
            get_array_prop(detail_props, "ReserveStockpileInfoList")
        {
            for reserve in reserve_structs {
                if let StructValue::Struct(reserve_props) = reserve {
                    let reserve_name =
                        get_string_prop(reserve_props, "StockpileName").unwrap_or_default();
                    let reserve_info = get_struct_prop(reserve_props, "StockpileInfo");
                    let reserve_items = reserve_info.map(parse_stockpile_items).unwrap_or_default();

                    result.push(Stockpile {
                        name: reserve_name,
                        stockpile_type: stockpile_type.clone(),
                        faction,
                        hex: map_id.clone(),
                        coords: coords.clone(),
                        is_reserve: true,
                        items: reserve_items,
                        tech: None,
                        timestamp,
                        shard: None,
                        ingame_timestamp: None,
                        resolution: None,
                        errors: timestamp_error.clone(),
                    });
                }
            }
        }
    }

    Ok(result)
}

/// Parse stockpile items from StockpileInfo struct.
fn parse_stockpile_items(value: &StructValue) -> Vec<StockpileItem> {
    let mut items = Vec::new();

    let props = match value {
        StructValue::Struct(props) => props,
        _ => return items,
    };

    // Helper to parse item arrays
    let parse_items = |array_name: &str, crated: bool| -> Vec<StockpileItem> {
        get_array_prop(props, array_name)
            .map(|arr| {
                if let ValueVec::Struct(item_structs) = arr {
                    let mut group: Vec<_> = item_structs
                        .iter()
                        .filter_map(|item| {
                            if let StructValue::Struct(item_props) = item {
                                let code =
                                    get_string_prop(item_props, "CodeName").unwrap_or_default();
                                let quantity = get_int32_prop(item_props, "Quantity").unwrap_or(0);
                                Some(StockpileItem::new(code, quantity, crated))
                            } else {
                                None
                            }
                        })
                        .collect();
                    // Sort by quantity descending
                    group.sort_by_key(|item| std::cmp::Reverse(item.quantity));
                    group
                } else {
                    vec![]
                }
            })
            .unwrap_or_default()
    };

    // Parse all item categories in order
    items.extend(parse_items("Items", false));
    items.extend(parse_items("ItemCrates", true));
    items.extend(parse_items("Vehicles", false));
    items.extend(parse_items("VehicleCrates", true));
    items.extend(parse_items("Structures", false));
    items.extend(parse_items("StructureCrates", true));

    items
}

/// Convert UE ticks to DateTime. Returns None for missing (zero) or
/// out-of-range ticks so callers can surface the failure rather than
/// substituting a fabricated "now".
fn parse_ue_timestamp(ticks: i64) -> Option<DateTime<Utc>> {
    if ticks == 0 {
        return None;
    }

    // UE ticks are 100-nanosecond intervals since 0001-01-01
    // Epoch ticks from 0001-01-01 to 1970-01-01
    const EPOCH_TICKS: i64 = 621355968000000000;
    let unix_ticks = ticks - EPOCH_TICKS;
    let unix_seconds = unix_ticks / 10_000_000;

    Utc.timestamp_opt(unix_seconds, 0).single()
}

// Helper functions to extract properties

fn get_string_prop(props: &Properties, name: &str) -> Option<String> {
    props.0.iter().find_map(|(key, prop)| {
        if key.1 == name {
            match prop {
                Property::Str(s) | Property::Name(s) => Some(s.clone()),
                Property::Enum(e) => Some(e.clone()),
                _ => None,
            }
        } else {
            None
        }
    })
}

fn get_int32_prop(props: &Properties, name: &str) -> Option<i32> {
    props.0.iter().find_map(|(key, prop)| {
        if key.1 == name {
            // Foxhole stores integer fields with varying property types
            // (e.g. UInt32Property for Quantity), so accept any integer variant.
            return match prop {
                Property::Int(v) => Some(*v),
                Property::Int8(v) => Some(*v as i32),
                Property::Int16(v) => Some(*v as i32),
                Property::Int64(v) => Some(*v as i32),
                Property::UInt8(v) => Some(*v as i32),
                Property::UInt16(v) => Some(*v as i32),
                Property::UInt32(v) => Some(*v as i32),
                Property::UInt64(v) => Some(*v as i32),
                Property::Byte(uesave::Byte::Byte(v)) => Some(*v as i32),
                _ => None,
            };
        }
        None
    })
}

/// Extract UE ticks from a DateTime struct property (e.g. `LastUpdated`).
/// Foxhole stores these as `StructValue::DateTime(u64)`, not a plain integer.
fn get_datetime_ticks_prop(props: &Properties, name: &str) -> Option<i64> {
    match get_struct_prop(props, name)? {
        StructValue::DateTime(ticks) => Some(*ticks as i64),
        _ => None,
    }
}

fn get_float_prop(props: &Properties, name: &str) -> Option<f64> {
    props.0.iter().find_map(|(key, prop)| {
        if key.1 == name {
            match prop {
                Property::Float(v) => Some(v.0 as f64),
                Property::Double(v) => Some(v.0),
                _ => None,
            }
        } else {
            None
        }
    })
}

fn get_struct_prop<'a>(props: &'a Properties, name: &str) -> Option<&'a StructValue> {
    props.0.iter().find_map(|(key, prop)| {
        if key.1 == name {
            if let Property::Struct(value) = prop {
                return Some(value);
            }
        }
        None
    })
}

fn get_byte_array_prop<'a>(props: &'a Properties, name: &str) -> Option<&'a [u8]> {
    match get_array_prop(props, name)? {
        ValueVec::Byte(ByteArray::Byte(bytes)) => Some(bytes),
        _ => None,
    }
}

fn get_array_prop<'a>(props: &'a Properties, name: &str) -> Option<&'a ValueVec> {
    props.0.iter().find_map(|(key, prop)| {
        if key.1 == name {
            if let Property::Array(value) = prop {
                return Some(value);
            }
        }
        None
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_parse_ue_timestamp() {
        // Zero ticks means "missing" — no fabricated fallback.
        assert!(parse_ue_timestamp(0).is_none());

        // Test a known timestamp (approximately 2024-01-01 00:00:00 UTC)
        let ticks: i64 = 638392320000000000;
        let parsed = parse_ue_timestamp(ticks).expect("valid ticks parse");
        assert!(parsed.year() >= 2023 && parsed.year() <= 2025);
    }
}
