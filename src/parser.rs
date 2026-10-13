//! SAV file parser using uesave library.

use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::Path;

use chrono::{DateTime, TimeZone, Utc};
use uesave::{ByteArray, Properties, Property, Save, SaveReader, StructValue, ValueVec};

use crate::error::{FsSavError, Result};
use crate::models::{
    factory_item_code, factory_queue_type, parse_tech, refinery_recipe_code, Faction, FactoryOrder,
    FactoryOrderItem, ParseResult, ParsedStockpiles, Refinery, RefineryAccess, RefineryAccessLevel,
    RefineryOrder, RefineryStorage, Stockpile, StockpileCoords, StockpileItem,
};

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
    let parsed = extract_stockpiles(&save);

    Ok(ParseResult {
        parsed_at: Utc::now(),
        save_file: path.display().to_string(),
        save_file_modified: modified,
        stockpiles: parsed.stockpiles,
        warnings: parsed.warnings,
    })
}

/// Parse a .sav file from bytes.
pub fn parse_save_bytes(data: &[u8]) -> Result<ParsedStockpiles> {
    let mut cursor = Cursor::new(data);

    let save = SaveReader::new()
        .error_to_raw(true)
        .read(&mut cursor)
        .map_err(|e| FsSavError::ParseError(e.to_string()))?;

    Ok(extract_stockpiles(&save))
}

/// Extract stockpiles from parsed save data, along with warnings about
/// tooltips that yielded none.
fn extract_stockpiles(save: &Save) -> ParsedStockpiles {
    let mut stockpiles = Vec::new();
    let mut warnings = Vec::new();

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
            if name_matches(&key.1, prop_name) {
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
                match parse_tooltip(props, faction, &mut warnings) {
                    Ok(mut parsed) => stockpiles.append(&mut parsed),
                    // Report it but keep parsing the other tooltips
                    Err(e) => warnings.push(format!("failed to parse tooltip: {e}")),
                }
            }
        }
    }

    ParsedStockpiles {
        stockpiles,
        warnings,
    }
}

/// `Flags` bit set when the details carry no data: either they were withheld
/// because the player was not on the structure's side, or the game had no
/// details for the structure (not found, or the tooltip could not be matched
/// to it). Nothing else is filled in then.
const DETAILS_UNAVAILABLE: i32 = 0x4;

/// `Flags` bit set on details the game actually provided for the structure.
/// Together with [`DETAILS_UNAVAILABLE`] it means they were withheld.
const DETAILS_PROVIDED: i32 = 0x80;

/// Parse a single tooltip into stockpiles (main + reserves). A tooltip whose
/// latest details carry no data yields no stockpiles and adds a warning.
fn parse_tooltip(
    props: &Properties,
    faction: Faction,
    warnings: &mut Vec<String>,
) -> Result<Vec<Stockpile>> {
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

    // Only the latest details (RecentMapItemDetails) are read: the initial
    // snapshot keeps the state from when the tooltip was first shown.
    let details = get_struct_prop(props, "RecentMapItemDetails");

    if let Some(StructValue::Struct(detail_props)) = details {
        let flags = get_int32_prop(detail_props, "Flags").unwrap_or(0);
        if flags & DETAILS_UNAVAILABLE != 0 {
            let reason = if flags & DETAILS_PROVIDED != 0 {
                "its latest details were withheld (not on the structure's side)"
            } else {
                "the game had no details for it when it was last updated"
            };
            warnings.push(format!(
                "{stockpile_type} at {} skipped: {reason}",
                map_id.as_deref().unwrap_or("unknown hex")
            ));
            return Ok(result);
        }

        // Parse main stockpile items
        let stockpile_info = get_struct_prop(detail_props, "StockpileInfo");
        let items = stockpile_info
            .map(parse_stockpile_items)
            .unwrap_or_default();

        // Garrison/upgrade tech build progress (bases only)
        let tech = get_byte_array_prop(detail_props, "Values")
            .and_then(|values| parse_tech(&stockpile_type, values));

        // Refinery queue and stored output
        let refinery = parse_refinery(detail_props, &stockpile_type);

        // The viewing player's factory orders
        let factory_orders = parse_factory_orders(detail_props, &stockpile_type);

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
            refinery,
            factory_orders,
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
                        refinery: None,
                        factory_orders: None,
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

/// Parse the refinery production state from a tooltip details struct.
///
/// - `RefineryOrderAccessLevel` (`Level` enum, `SquadId` int) +
///   `RefineryOrders`: the queue the viewing player had selected and its
///   recipes (`Index` u8, `Source`/`Refined`/`Duration` u16).
/// - `RefineryStorages`: refined output held per owner (`AccessLevel`) and
///   recipe slot (`Index` u8, `Refined` u16).
///
/// Every tooltip carries a default `RefineryOrderAccessLevel`, so `None` is
/// returned unless there is at least one order or storage entry.
fn parse_refinery(detail_props: &Properties, stockpile_type: &str) -> Option<Refinery> {
    let recipe_code =
        |slot: u8| refinery_recipe_code(stockpile_type, slot).map(|code| code.to_string());

    let orders: Vec<RefineryOrder> = get_struct_array(detail_props, "RefineryOrders")
        .filter_map(|order| {
            let slot = get_u8_prop(order, "Index")?;
            Some(RefineryOrder {
                slot,
                code: recipe_code(slot),
                source: get_u16_prop(order, "Source").unwrap_or(0),
                refined: get_u16_prop(order, "Refined").unwrap_or(0),
                duration: get_u16_prop(order, "Duration").unwrap_or(0),
            })
        })
        .collect();

    let storages: Vec<RefineryStorage> = get_struct_array(detail_props, "RefineryStorages")
        .filter_map(|storage| {
            let access = parse_refinery_access(get_struct_prop(storage, "AccessLevel")?)?;
            let slot = get_u8_prop(storage, "Index")?;
            Some(RefineryStorage {
                access,
                slot,
                code: recipe_code(slot),
                refined: get_u16_prop(storage, "Refined").unwrap_or(0),
            })
        })
        .collect();

    if orders.is_empty() && storages.is_empty() {
        return None;
    }

    Some(Refinery {
        orders_access: get_struct_prop(detail_props, "RefineryOrderAccessLevel")
            .and_then(parse_refinery_access),
        orders,
        storages,
    })
}

/// Parse the viewing player's factory orders from a tooltip details struct
/// (`FactoryEntries`: `QueueIndex` u8, `Duration` u16, `CodeNames` u8 array of
/// item positions in the queue's production list). `None` when there are none.
fn parse_factory_orders(
    detail_props: &Properties,
    stockpile_type: &str,
) -> Option<Vec<FactoryOrder>> {
    let orders: Vec<FactoryOrder> = get_struct_array(detail_props, "FactoryEntries")
        .filter_map(|entry| {
            let queue = get_u8_prop(entry, "QueueIndex")?;
            let items = get_byte_array_prop(entry, "CodeNames")
                .unwrap_or_default()
                .iter()
                .map(|&slot| FactoryOrderItem {
                    slot,
                    code: factory_item_code(stockpile_type, queue, slot).map(str::to_string),
                })
                .collect();
            Some(FactoryOrder {
                queue,
                queue_type: factory_queue_type(stockpile_type, queue).map(str::to_string),
                duration: get_u16_prop(entry, "Duration").unwrap_or(0),
                items,
            })
        })
        .collect();

    (!orders.is_empty()).then_some(orders)
}

/// Parse a `{Level, SquadId}` refinery access struct. The squad id is only
/// meaningful (and only kept) for the squad access level.
fn parse_refinery_access(value: &StructValue) -> Option<RefineryAccess> {
    let StructValue::Struct(props) = value else {
        return None;
    };
    let level = RefineryAccessLevel::from_save_value(&get_string_prop(props, "Level")?)?;
    let squad_id = match level {
        RefineryAccessLevel::Squad => get_int32_prop(props, "SquadId"),
        _ => None,
    };
    Some(RefineryAccess { level, squad_id })
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

/// Property names are case-insensitive: the save may spell a name with a
/// different case than the structure declares it (e.g. `SquadId`).
fn name_matches(key: &str, name: &str) -> bool {
    key.eq_ignore_ascii_case(name)
}

fn get_string_prop(props: &Properties, name: &str) -> Option<String> {
    props.0.iter().find_map(|(key, prop)| {
        if name_matches(&key.1, name) {
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
        if name_matches(&key.1, name) {
            // Foxhole stores integer fields with varying property types
            // (e.g. Int16Property for item Quantity, UInt16Property for
            // refinery counts, ByteProperty for indexes), so accept any
            // integer variant.
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
        if name_matches(&key.1, name) {
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
        if name_matches(&key.1, name) {
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
        if name_matches(&key.1, name) {
            if let Property::Array(value) = prop {
                return Some(value);
            }
        }
        None
    })
}

/// Iterate the struct elements of an array property (empty if absent).
fn get_struct_array<'a>(
    props: &'a Properties,
    name: &str,
) -> impl Iterator<Item = &'a Properties> + 'a {
    let elements = match get_array_prop(props, name) {
        Some(ValueVec::Struct(elements)) => elements.as_slice(),
        _ => &[],
    };
    elements.iter().filter_map(|element| match element {
        StructValue::Struct(element_props) => Some(element_props),
        _ => None,
    })
}

fn get_u8_prop(props: &Properties, name: &str) -> Option<u8> {
    get_int32_prop(props, name).and_then(|v| u8::try_from(v).ok())
}

fn get_u16_prop(props: &Properties, name: &str) -> Option<u16> {
    get_int32_prop(props, name).and_then(|v| u16::try_from(v).ok())
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

    /// A `FactoryEntries` entry as the save stores it.
    fn factory_entry(queue: u8, duration: u16, slots: &[u8]) -> StructValue {
        let mut entry = Properties::default();
        entry.insert("QueueIndex", Property::Byte(uesave::Byte::Byte(queue)));
        entry.insert("Duration", Property::UInt16(duration));
        entry.insert(
            "CodeNames",
            Property::Array(ValueVec::Byte(ByteArray::Byte(slots.to_vec()))),
        );
        StructValue::Struct(entry)
    }

    fn details_with_factory_entries(entries: Vec<StructValue>) -> Properties {
        let mut details = Properties::default();
        details.insert("FactoryEntries", Property::Array(ValueVec::Struct(entries)));
        details
    }

    #[test]
    fn test_parse_factory_orders() {
        let details = details_with_factory_entries(vec![
            // SmallArms: RifleC (slot 16) ready to collect
            factory_entry(0, 0, &[16]),
            // Uniforms: two items, 90 s left
            factory_entry(6, 90, &[13, 15]),
        ]);

        let orders = parse_factory_orders(&details, "Factory").expect("orders");
        assert_eq!(orders.len(), 2);

        assert_eq!(orders[0].queue, 0);
        assert_eq!(orders[0].queue_type.as_deref(), Some("SmallArms"));
        assert_eq!(orders[0].duration, 0);
        assert_eq!(orders[0].items.len(), 1);
        assert_eq!(orders[0].items[0].slot, 16);
        assert_eq!(orders[0].items[0].code.as_deref(), Some("RifleC"));

        assert_eq!(orders[1].queue_type.as_deref(), Some("Uniforms"));
        assert_eq!(orders[1].duration, 90);
        let codes: Vec<_> = orders[1].items.iter().map(|i| i.code.as_deref()).collect();
        assert_eq!(codes, [Some("ScoutUniformC"), Some("SnowUniformC")]);
    }

    #[test]
    fn test_parse_factory_orders_queues_differ_per_structure() {
        // Queue 3 is Utility in a Factory but Supplies in a Mass Production
        // Factory, and HeavyAmmo lists its items in a different order.
        let details = details_with_factory_entries(vec![
            factory_entry(3, 10, &[0]),
            factory_entry(2, 10, &[10]),
        ]);

        let factory = parse_factory_orders(&details, "Factory").unwrap();
        assert_eq!(factory[0].queue_type.as_deref(), Some("Utility"));
        assert_eq!(
            factory[0].items[0].code.as_deref(),
            Some("BarbedwireMaterials")
        );
        assert_eq!(factory[1].items[0].code.as_deref(), Some("ATAmmo"));

        let mpf = parse_factory_orders(&details, "MassProduction").unwrap();
        assert_eq!(mpf[0].queue_type.as_deref(), Some("Supplies"));
        assert_eq!(mpf[0].items[0].code.as_deref(), Some("MaintenanceSupplies"));
        assert_eq!(mpf[1].items[0].code.as_deref(), Some("WaterWallMaterials"));
    }

    #[test]
    fn test_parse_factory_orders_unknown_slots_and_structures() {
        let details = details_with_factory_entries(vec![factory_entry(0, 5, &[255])]);

        // An item not in the queue's list is reported by slot only.
        let orders = parse_factory_orders(&details, "Factory").unwrap();
        assert_eq!(orders[0].items[0].slot, 255);
        assert_eq!(orders[0].items[0].code, None);

        // Queues of unknown structures keep their raw positions.
        let unknown = parse_factory_orders(&details, "SomeNewFactory").unwrap();
        assert_eq!(unknown[0].queue_type, None);
        assert_eq!(unknown[0].items[0].code, None);
    }

    #[test]
    fn test_parse_factory_orders_none_when_empty() {
        assert!(parse_factory_orders(&details_with_factory_entries(vec![]), "Factory").is_none());
        assert!(parse_factory_orders(&Properties::default(), "Factory").is_none());
    }
}
