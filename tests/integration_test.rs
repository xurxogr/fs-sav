//! Integration tests for fs-sav parser.

use fs_sav::parse_save;

const TEST_SAV_PATH: &str = "tests/fixtures/test.sav";

#[test]
fn test_parse_test_sav() {
    let result = parse_save(TEST_SAV_PATH).expect("Failed to parse test.sav");

    // Should find 26 stockpiles
    assert_eq!(result.stockpiles.len(), 26);

    // All stockpiles should have a hex
    for stockpile in &result.stockpiles {
        assert!(stockpile.hex.is_some(), "Stockpile should have hex");
        assert!(stockpile.coords.is_some(), "Stockpile should have coords");
    }
}

#[test]
fn test_stockpile_types_coverage() {
    let result = parse_save(TEST_SAV_PATH).expect("Failed to parse test.sav");

    // Collect all unique types (raw in-game CodeNames)
    let types: std::collections::HashSet<_> = result
        .stockpiles
        .iter()
        .map(|s| s.stockpile_type.as_str())
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

    // Collect all unique hex names
    let hexes: std::collections::HashSet<_> = result
        .stockpiles
        .iter()
        .filter_map(|s| s.hex.as_deref())
        .collect();

    // Should have various hexes
    assert!(hexes.contains("TerminusHex"));
    assert!(hexes.contains("ReaversPassHex"));
    assert!(hexes.contains("DeadLandsHex"));
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
