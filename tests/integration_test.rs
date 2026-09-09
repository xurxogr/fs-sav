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

const TEST_REFINERY_SAV_PATH: &str = "tests/fixtures/refinery_mapdata.sav";

#[test]
fn test_refinery_squad_queue() {
    let result = parse_save(TEST_REFINERY_SAV_PATH).expect("Failed to parse fixture");

    let squad_queues: Vec<_> = result
        .stockpiles
        .iter()
        .filter(|s| s.access_level.as_deref() == Some("squad"))
        .collect();
    assert_eq!(squad_queues.len(), 1, "expected one squad queue");

    let queue = squad_queues[0];
    assert_eq!(queue.squad_id, Some(1));
    assert_eq!(queue.stockpile_type, "Refinery");
    assert_eq!(queue.hex.as_deref(), Some("TerminusHex"));
    assert!(queue.is_reserve, "squad queues are reserve stockpiles");
    assert_eq!(queue.name, "squad:1");

    // 7 bays carry orders; slot 3 has no order and must be absent.
    assert_eq!(queue.items.len(), 7);

    // The recent details snapshot wins over the initial one: slot 0 shows
    // 10 (recent), not 18060 (initial).
    let find = |code: &str| queue.items.iter().find(|i| i.code == code).unwrap();
    assert_eq!(find("Cloth").quantity, 10);
    assert_eq!(find("Diesel").quantity, 5309);
    assert_eq!(find("Explosive").quantity, 3295);
    assert_eq!(find("HeavyExplosive").quantity, 0);
    assert_eq!(find("Wood").quantity, 0);
    assert_eq!(find("GroundMaterials").quantity, 0);
    assert_eq!(find("IronA").quantity, 0);
}

#[test]
fn test_refinery_public_queue() {
    let result = parse_save(TEST_REFINERY_SAV_PATH).expect("Failed to parse fixture");

    let public_queues: Vec<_> = result
        .stockpiles
        .iter()
        .filter(|s| s.access_level.as_deref() == Some("public"))
        .collect();
    assert_eq!(public_queues.len(), 1, "expected one public queue");

    let queue = public_queues[0];
    assert_eq!(queue.squad_id, None);
    assert_eq!(queue.stockpile_type, "Refinery");
    assert!(
        !queue.is_reserve,
        "public queues are not reserve stockpiles"
    );
    assert_eq!(queue.name, "public");
    assert_eq!(queue.items.len(), 1);
    assert_eq!(queue.items[0].code, "Cloth");
    assert_eq!(queue.items[0].quantity, 50);
}

#[test]
fn test_refinery_storage_stockpile_has_no_access_level() {
    let result = parse_save(TEST_REFINERY_SAV_PATH).expect("Failed to parse fixture");

    // The refinery's public storage (uncollected output) is emitted by the
    // existing tooltip logic and must not carry queue fields.
    let storages: Vec<_> = result
        .stockpiles
        .iter()
        .filter(|s| s.stockpile_type == "Refinery" && s.access_level.is_none())
        .collect();
    assert!(!storages.is_empty(), "refinery storage must be present");
    for storage in storages {
        assert_eq!(storage.squad_id, None);
    }
}

#[test]
fn test_refinery_queue_items_sorted_by_slot() {
    let result = parse_save(TEST_REFINERY_SAV_PATH).expect("Failed to parse fixture");

    let queue = result
        .stockpiles
        .iter()
        .find(|s| s.access_level.as_deref() == Some("squad"))
        .expect("squad queue");

    // Items keep the in-game production slot order.
    let expected_order = [
        "Cloth",
        "Diesel",
        "Explosive",
        "Wood",
        "HeavyExplosive",
        "GroundMaterials",
        "IronA",
    ];
    let actual: Vec<_> = queue.items.iter().map(|i| i.code.as_str()).collect();
    assert_eq!(actual, expected_order);
}
