//! Integration tests for fs-sav parser.

use fs_sav::parse_save;

const TEST_SAV_PATH: &str = "tests/fixtures/test.sav";

#[test]
fn test_parse_test_sav() {
    let result = parse_save(TEST_SAV_PATH).expect("Failed to parse test.sav");

    // Only TownBase3 has current details; the other 25 pinned tooltips were
    // last updated without details (the game had none for them) and are
    // skipped.
    assert_eq!(result.stockpiles.len(), 1);
    assert_eq!(result.warnings.len(), 25);
    assert!(result
        .warnings
        .iter()
        .all(|w| w.contains("the game had no details for it")));

    // All stockpiles should have a hex
    for stockpile in &result.stockpiles {
        assert!(stockpile.hex.is_some(), "Stockpile should have hex");
        assert!(stockpile.coords.is_some(), "Stockpile should have coords");
    }
}

#[test]
fn test_unavailable_tooltip_tech_not_read_from_initial_details() {
    let result = parse_save(TEST_SAV_PATH).expect("Failed to parse test.sav");

    // TownBase1's initial snapshot has tech values, but its latest details
    // carry no data: it must be skipped, not reported with stale tech.
    assert!(result
        .stockpiles
        .iter()
        .all(|s| s.stockpile_type != "TownBase1"));

    // TownBase3's latest details are current and carry its tech.
    let townbase3 = result
        .stockpiles
        .iter()
        .find(|s| s.stockpile_type == "TownBase3")
        .expect("Should have TownBase3");
    let tech = townbase3.tech.as_ref().expect("TownBase3 should have tech");
    assert_eq!(tech.large_garrison, Some(100));
    assert_eq!(tech.fortifications, Some(100));
}

#[test]
fn test_stockpile_types_coverage() {
    let result = parse_save(TEST_SAV_PATH).expect("Failed to parse test.sav");

    // Every pinned tooltip is accounted for, either as a stockpile or as a
    // skipped-tooltip warning (which starts with the raw in-game CodeName).
    let types: std::collections::HashSet<_> = result
        .stockpiles
        .iter()
        .map(|s| s.stockpile_type.as_str())
        .chain(
            result
                .warnings
                .iter()
                .filter_map(|w| w.split_whitespace().next()),
        )
        .collect();

    // Should have all expected stockpile types
    for expected in [
        "GarrisonStation",
        "Keep",
        "ForwardBase1",
        "RelicBase1",
        "FortBaseT1",
        "FortBaseT2",
        "FortBaseT3",
        "BorderBase",
        "TownBase1",
        "TownBase2",
        "TownBase3",
        "FortGarrisonStation",
        "StorageFacility",
        "Seaport",
        "AircraftDepot",
        "Hospital",
        "Refinery",
        "MaintenanceTunnel",
        "FacilityFactorySmallArms",
        "FacilityModificationCenter",
        "FacilityTransferLiquid",
        "FacilityTransferMaterial",
        "FacilityTransferResource",
        "FacilityVehicleFactory1",
        "FacilityVehicleFactory2",
        "FacilityVehicleFactory3",
    ] {
        assert!(types.contains(expected), "missing type: {expected}");
    }
}

#[test]
fn test_stockpile_with_items() {
    let result = parse_save(TEST_SAV_PATH).expect("Failed to parse test.sav");

    // Find TownBase3 which has items
    let townbase3 = result
        .stockpiles
        .iter()
        .find(|s| s.stockpile_type == "TownBase3")
        .expect("Should have TownBase3");

    // Should have items
    assert!(!townbase3.items.is_empty(), "TownBase3 should have items");
    assert_eq!(townbase3.items.len(), 29);

    // Check some known items
    let item_codes: Vec<_> = townbase3.items.iter().map(|i| i.code.as_str()).collect();
    assert!(item_codes.contains(&"RifleC"));
    assert!(item_codes.contains(&"RifleAmmo"));
    assert!(item_codes.contains(&"SoldierSupplies"));
}

#[test]
fn test_hex_names() {
    let result = parse_save(TEST_SAV_PATH).expect("Failed to parse test.sav");

    // Hex names are cleaned from the save enum value
    // ("EWorldConquestMapId::DeadLandsHex" -> "DeadLandsHex").
    let hexes: std::collections::HashSet<_> = result
        .stockpiles
        .iter()
        .filter_map(|s| s.hex.as_deref())
        .collect();
    assert_eq!(hexes, std::collections::HashSet::from(["DeadLandsHex"]));

    // Skipped tooltips name their hex in the warning.
    for hex in ["TerminusHex", "ReaversPassHex", "DeadLandsHex"] {
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.contains(&format!(" at {hex} "))),
            "no warning for {hex}"
        );
    }
}

#[test]
fn test_coordinates_valid() {
    let result = parse_save(TEST_SAV_PATH).expect("Failed to parse test.sav");

    for stockpile in &result.stockpiles {
        if let Some(coords) = &stockpile.coords {
            // Coordinates should be normalized (0.0 to 1.0)
            assert!(
                coords.x >= 0.0 && coords.x <= 1.0,
                "X coordinate should be normalized: {}",
                coords.x
            );
            assert!(
                coords.y >= 0.0 && coords.y <= 1.0,
                "Y coordinate should be normalized: {}",
                coords.y
            );
        }
    }
}

const TEST_REFINERY_SAV_PATH: &str = "tests/fixtures/refinery_mapdata.sav";

#[test]
fn test_refinery_orders_and_storages() {
    use fs_sav::models::{RefineryAccess, RefineryAccessLevel};

    let result = parse_save(TEST_REFINERY_SAV_PATH).expect("Failed to parse fixture");

    let refineries: Vec<_> = result
        .stockpiles
        .iter()
        .filter_map(|s| s.refinery.as_ref().map(|r| (s, r)))
        .collect();
    assert_eq!(refineries.len(), 1, "only one tooltip has refinery data");

    let (stockpile, refinery) = refineries[0];
    assert_eq!(stockpile.stockpile_type, "Refinery");
    assert_eq!(stockpile.hex.as_deref(), Some("TerminusHex"));
    assert!(!stockpile.is_reserve);

    // The player had the public queue selected, with one finished order.
    assert_eq!(
        refinery.orders_access,
        Some(RefineryAccess {
            level: RefineryAccessLevel::Public,
            squad_id: None,
        })
    );
    assert_eq!(refinery.orders.len(), 1);
    let order = &refinery.orders[0];
    assert_eq!(order.slot, 0);
    assert_eq!(order.code.as_deref(), Some("Cloth"));
    assert_eq!(order.source, 0);
    assert_eq!(order.refined, 50);
    assert_eq!(order.duration, 0);

    // Output held by squad 1, read from the recent snapshot (slot 0 is 10,
    // not the 18060 of the initial snapshot).
    let squad = RefineryAccess {
        level: RefineryAccessLevel::Squad,
        squad_id: Some(1),
    };
    assert_eq!(refinery.storages.len(), 7);
    assert!(refinery.storages.iter().all(|s| s.access == squad));
    let refined = |slot: u8| {
        refinery
            .storages
            .iter()
            .find(|s| s.slot == slot)
            .map(|s| s.refined)
    };
    assert_eq!(refined(0), Some(10));
    assert_eq!(refined(1), Some(5309));
    assert_eq!(refined(2), Some(3295));
    assert_eq!(refined(3), None);
}

#[test]
fn test_refinery_data_does_not_create_stockpiles() {
    let result = parse_save(TEST_REFINERY_SAV_PATH).expect("Failed to parse fixture");

    // Refinery data is attached to the structure's main stockpile only.
    assert!(result
        .stockpiles
        .iter()
        .filter(|s| s.is_reserve)
        .all(|s| s.refinery.is_none()));
    assert!(result
        .stockpiles
        .iter()
        .filter(|s| s.stockpile_type != "Refinery")
        .all(|s| s.refinery.is_none()));
}

#[test]
fn test_parse_save_bytes_reports_warnings() {
    let data = std::fs::read(TEST_SAV_PATH).expect("Failed to read test.sav");
    let parsed = fs_sav::parse_save_bytes(&data).expect("Failed to parse test.sav");
    let from_file = parse_save(TEST_SAV_PATH).expect("Failed to parse test.sav");

    // Bytes and file parsing yield the same stockpiles and warnings.
    assert_eq!(parsed.stockpiles, from_file.stockpiles);
    assert_eq!(parsed.warnings, from_file.warnings);
    assert_eq!(parsed.warnings.len(), 25);
}
