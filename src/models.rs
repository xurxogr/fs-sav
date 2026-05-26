//! Data models matching the foxhole-stockpiles Python schema.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Stockpile type enum matching in-game CodeNames.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum StockpileType {
    // Bases
    #[serde(rename = "GarrisonStation")]
    Encampment,
    #[serde(rename = "Keep")]
    Keep,
    #[serde(rename = "ForwardBase1")]
    SafeHouse,
    #[serde(rename = "RelicBase1")]
    RelicBase,
    #[serde(rename = "FortBaseT1")]
    BunkerBase1,
    #[serde(rename = "FortBaseT2")]
    BunkerBase2,
    #[serde(rename = "FortBaseT3")]
    BunkerBase3,
    #[serde(rename = "BorderBase")]
    BorderBase,
    #[serde(rename = "TownBase1")]
    TownBase1,
    #[serde(rename = "TownBase2")]
    TownBase2,
    #[serde(rename = "TownBase3")]
    TownBase3,
    #[serde(rename = "FortGarrisonStation")]
    UndergroundFortress,
    #[serde(rename = "LargeShipBaseShip")]
    BmsLonghook,
    #[serde(rename = "LargeShipStorageShip")]
    BmsBluefin,

    // Structures
    #[serde(rename = "StorageFacility")]
    StorageDepot,
    #[serde(rename = "Seaport")]
    Seaport,
    #[serde(rename = "AircraftDepot")]
    AircraftDepot,

    // Facilities
    #[serde(rename = "Hospital")]
    Hospital,
    #[serde(rename = "Refinery")]
    Refinery,
    #[serde(rename = "MaintenanceTunnel")]
    MaintenanceTunnel,
    #[serde(rename = "FacilityFactorySmallArms")]
    SmallArmsFactory,
    #[serde(rename = "FacilityModificationCenter")]
    ModificationCenter,
    #[serde(rename = "FacilityTransferLiquid")]
    TransferLiquid,
    #[serde(rename = "FacilityTransferMaterial")]
    TransferMaterial,
    #[serde(rename = "FacilityTransferResource")]
    TransferResource,
    #[serde(rename = "FacilityVehicleFactory1")]
    VehicleFactory1,
    #[serde(rename = "FacilityVehicleFactory2")]
    VehicleFactory2,
    #[serde(rename = "FacilityVehicleFactory3")]
    VehicleFactory3,

    #[default]
    #[serde(rename = "Undefined")]
    Undefined,
}

impl StockpileType {
    /// Parse from in-game CodeName string.
    pub fn from_code_name(code: &str) -> Self {
        match code {
            "GarrisonStation" => Self::Encampment,
            "Keep" => Self::Keep,
            "ForwardBase1" => Self::SafeHouse,
            "RelicBase1" => Self::RelicBase,
            "FortBaseT1" => Self::BunkerBase1,
            "FortBaseT2" => Self::BunkerBase2,
            "FortBaseT3" => Self::BunkerBase3,
            "BorderBase" => Self::BorderBase,
            "TownBase1" => Self::TownBase1,
            "TownBase2" => Self::TownBase2,
            "TownBase3" => Self::TownBase3,
            "FortGarrisonStation" => Self::UndergroundFortress,
            "LargeShipBaseShip" => Self::BmsLonghook,
            "LargeShipStorageShip" => Self::BmsBluefin,
            "StorageFacility" => Self::StorageDepot,
            "Seaport" => Self::Seaport,
            "AircraftDepot" => Self::AircraftDepot,
            "Hospital" => Self::Hospital,
            "Refinery" => Self::Refinery,
            "MaintenanceTunnel" => Self::MaintenanceTunnel,
            "FacilityFactorySmallArms" => Self::SmallArmsFactory,
            "FacilityModificationCenter" => Self::ModificationCenter,
            "FacilityTransferLiquid" => Self::TransferLiquid,
            "FacilityTransferMaterial" => Self::TransferMaterial,
            "FacilityTransferResource" => Self::TransferResource,
            "FacilityVehicleFactory1" => Self::VehicleFactory1,
            "FacilityVehicleFactory2" => Self::VehicleFactory2,
            "FacilityVehicleFactory3" => Self::VehicleFactory3,
            _ => Self::Undefined,
        }
    }
}

/// Faction that owns a stockpile, derived from which pinned-tooltips
/// property the data was read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Faction {
    #[serde(rename = "Warden")]
    Warden,
    #[serde(rename = "Colonial")]
    Colonial,
}

/// Normalized map coordinates for a stockpile location.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StockpileCoords {
    /// Normalized X coordinate on the map (0.0 to 1.0)
    pub x: f64,
    /// Normalized Y coordinate on the map (0.0 to 1.0)
    pub y: f64,
}

/// A single item in a stockpile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StockpileItem {
    /// Item code name
    pub code: String,
    /// Quantity of the item
    pub quantity: i32,
    /// Whether the item is crated
    pub crated: bool,
    /// Confidence of detection (None for SAV-parsed items)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
}

impl StockpileItem {
    /// Create a new stockpile item from SAV data.
    pub fn new(code: String, quantity: i32, crated: bool) -> Self {
        Self {
            code,
            quantity,
            crated,
            confidence: None,
        }
    }
}

/// A stockpile containing items.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stockpile {
    /// Name of the stockpile (empty for public stockpiles)
    #[serde(default)]
    pub name: String,

    /// Type of stockpile
    #[serde(rename = "type")]
    pub stockpile_type: StockpileType,

    /// Faction that owns the stockpile ("Warden" or "Colonial")
    pub faction: Faction,

    /// Hex region name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hex: Option<String>,

    /// Map coordinates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coords: Option<StockpileCoords>,

    /// Whether this is a reserve stockpile
    #[serde(default)]
    pub is_reserve: bool,

    /// List of items in the stockpile
    #[serde(default)]
    pub items: Vec<StockpileItem>,

    /// Last update timestamp
    pub timestamp: DateTime<Utc>,

    /// Shard name (None for SAV-parsed stockpiles)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shard: Option<String>,

    /// In-game timestamp (None for SAV-parsed stockpiles)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ingame_timestamp: Option<String>,

    /// Resolution of screenshot (None for SAV-parsed stockpiles)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,

    /// Errors during processing (None for SAV-parsed stockpiles)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<String>>,
}

impl Stockpile {
    /// Generate a unique key for this stockpile (for change tracking).
    pub fn to_key(&self) -> String {
        let coords_key = self
            .coords
            .as_ref()
            .map(|c| format!("{:.6},{:.6}", c.x, c.y))
            .unwrap_or_else(|| "0,0".to_string());

        let type_str = serde_json::to_string(&self.stockpile_type)
            .unwrap_or_else(|_| "\"Undefined\"".to_string())
            .trim_matches('"')
            .to_string();

        format!(
            "{}:{}:{}:{}",
            type_str,
            self.hex.as_deref().unwrap_or(""),
            coords_key,
            self.name
        )
    }
}

/// Result of parsing a save file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseResult {
    /// When the file was parsed
    pub parsed_at: DateTime<Utc>,
    /// Path to the save file
    pub save_file: String,
    /// When the save file was last modified
    pub save_file_modified: DateTime<Utc>,
    /// List of stockpiles found
    pub stockpiles: Vec<Stockpile>,
    /// Any warnings during parsing
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

/// Parser metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserInfo {
    /// Implementation type
    pub implementation: String,
    /// Parser version
    pub version: String,
}

impl Default for ParserInfo {
    fn default() -> Self {
        Self {
            implementation: "rust".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stockpile_type_from_code_name() {
        assert_eq!(
            StockpileType::from_code_name("Seaport"),
            StockpileType::Seaport
        );
        assert_eq!(
            StockpileType::from_code_name("StorageFacility"),
            StockpileType::StorageDepot
        );
        assert_eq!(
            StockpileType::from_code_name("Unknown"),
            StockpileType::Undefined
        );
    }

    #[test]
    fn test_stockpile_type_serialization() {
        let st = StockpileType::Seaport;
        let json = serde_json::to_string(&st).unwrap();
        assert_eq!(json, "\"Seaport\"");
    }

    #[test]
    fn test_stockpile_to_key() {
        let stockpile = Stockpile {
            name: "Test".to_string(),
            stockpile_type: StockpileType::Seaport,
            faction: Faction::Warden,
            hex: Some("Westgate".to_string()),
            coords: Some(StockpileCoords { x: 0.5, y: 0.5 }),
            is_reserve: false,
            items: vec![],
            timestamp: Utc::now(),
            shard: None,
            ingame_timestamp: None,
            resolution: None,
            errors: None,
        };

        let key = stockpile.to_key();
        assert!(key.contains("Seaport"));
        assert!(key.contains("Westgate"));
        assert!(key.contains("Test"));
    }
}
