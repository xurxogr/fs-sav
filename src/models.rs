//! Data models matching the foxhole-stockpiles Python schema.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Ordered tech-tree garrison/upgrade slots for a structure, keyed by its
/// in-game CodeName and matching the positional `Values.Byte` array in the
/// save. Empty for structures (facilities, depots, ships) that have no tech,
/// and for any unrecognized CodeName.
fn tech_components(code_name: &str) -> &'static [TechComponent] {
    use TechComponent::*;
    match code_name {
        "ForwardBase1" | "Keep" => &[ProvisionalGarrison, SmallGarrison, LargeGarrison],
        "BorderBase" => &[ProvisionalGarrison],
        "RelicBase1" => &[
            ProvisionalGarrison,
            SmallGarrison,
            LargeGarrison,
            Fortifications,
        ],
        "GarrisonStation" => &[
            ProvisionalGarrison,
            SmallGarrison,
            LargeGarrison,
            RadioStation,
            ArtilleryShelter,
        ],
        "TownBase1" | "TownBase2" | "TownBase3" => &[
            ProvisionalGarrison,
            SmallGarrison,
            LargeGarrison,
            Industry,
            OccupiedTown,
            Fortifications,
        ],
        "FortBaseT1" | "FortBaseT2" | "FortBaseT3" => &[
            ProvisionalGarrison,
            SmallGarrison,
            LargeGarrison,
            T1Garrison,
            T2Garrison,
            T3Garrison,
            ArtilleryGarrison,
            T1SupportBunkers,
            T3SupportBunkers,
            Deployment,
            AdvancedBunkers,
        ],
        _ => &[],
    }
}

/// Decode a save `Values.Byte` array (build progress 0-100 per slot) into a
/// labeled [`Tech`] for the structure with the given in-game CodeName. Returns
/// `None` when the structure has no tech or the save carries no values.
pub fn parse_tech(code_name: &str, values: &[u8]) -> Option<Tech> {
    let components = tech_components(code_name);
    if components.is_empty() || values.is_empty() {
        return None;
    }

    let mut tech = Tech::default();
    for (component, &value) in components.iter().zip(values) {
        component.assign(&mut tech, value);
    }
    Some(tech)
}

/// A single garrison/upgrade tech component (a value of the in-game
/// `ETechComponentID` enum). Shared identifiers mean the same component across
/// structures; only their position in a structure's `Values` array differs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TechComponent {
    ProvisionalGarrison,
    SmallGarrison,
    LargeGarrison,
    T1Garrison,
    T2Garrison,
    T3Garrison,
    ArtilleryGarrison,
    T1SupportBunkers,
    T3SupportBunkers,
    Deployment,
    AdvancedBunkers,
    RadioStation,
    ArtilleryShelter,
    Industry,
    OccupiedTown,
    Fortifications,
}

impl TechComponent {
    fn assign(self, tech: &mut Tech, value: u8) {
        let slot = match self {
            Self::ProvisionalGarrison => &mut tech.provisional_garrison,
            Self::SmallGarrison => &mut tech.small_garrison,
            Self::LargeGarrison => &mut tech.large_garrison,
            Self::T1Garrison => &mut tech.t1_garrison,
            Self::T2Garrison => &mut tech.t2_garrison,
            Self::T3Garrison => &mut tech.t3_garrison,
            Self::ArtilleryGarrison => &mut tech.artillery_garrison,
            Self::T1SupportBunkers => &mut tech.t1_support_bunkers,
            Self::T3SupportBunkers => &mut tech.t3_support_bunkers,
            Self::Deployment => &mut tech.deployment,
            Self::AdvancedBunkers => &mut tech.advanced_bunkers,
            Self::RadioStation => &mut tech.radio_station,
            Self::ArtilleryShelter => &mut tech.artillery_shelter,
            Self::Industry => &mut tech.industry,
            Self::OccupiedTown => &mut tech.occupied_town,
            Self::Fortifications => &mut tech.fortifications,
        };
        *slot = Some(value);
    }
}

/// Garrison/upgrade tech build progress (0-100 per slot) for a base structure.
/// Each field is a tech component; only the slots that exist for a given
/// structure are populated. Fields are declared in an order that keeps every
/// structure's slots in tech-tree order.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Tech {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provisional_garrison: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_garrison: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub large_garrison: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub t1_garrison: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub t2_garrison: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub t3_garrison: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artillery_garrison: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub t1_support_bunkers: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub t3_support_bunkers: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advanced_bunkers: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radio_station: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artillery_shelter: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub industry: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub occupied_town: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fortifications: Option<u8>,
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

    /// Type of stockpile, as the in-game CodeName (e.g. "Seaport"). Kept as a
    /// free-form string so newly added in-game structure types remain valid
    /// rather than collapsing to "Undefined".
    #[serde(rename = "type")]
    pub stockpile_type: String,

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

    /// Garrison/upgrade tech build progress (bases only; None otherwise)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tech: Option<Tech>,

    /// Last update timestamp (None if absent/invalid in the save; see `errors`)
    pub timestamp: Option<DateTime<Utc>>,

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

        let type_str = if self.stockpile_type.is_empty() {
            "Undefined"
        } else {
            &self.stockpile_type
        };

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
    fn test_parse_tech_garrison_station() {
        // GarrisonStation: index 3 = RadioStation, 4 = ArtilleryShelter.
        let tech = parse_tech("GarrisonStation", &[100, 50, 0, 30, 0]).unwrap();
        assert_eq!(tech.provisional_garrison, Some(100));
        assert_eq!(tech.small_garrison, Some(50));
        assert_eq!(tech.large_garrison, Some(0));
        assert_eq!(tech.radio_station, Some(30));
        assert_eq!(tech.artillery_shelter, Some(0));
        // Slots that don't apply to this structure stay unset.
        assert_eq!(tech.fortifications, None);
        assert_eq!(tech.industry, None);
    }

    #[test]
    fn test_parse_tech_index3_differs_by_type() {
        // The 4th value is Fortifications for a RelicBase but RadioStation for
        // a GarrisonStation - position is per-type, meaning is shared.
        let relic = parse_tech("RelicBase1", &[100, 100, 100, 80]).unwrap();
        assert_eq!(relic.fortifications, Some(80));
        assert_eq!(relic.radio_station, None);

        let garrison = parse_tech("GarrisonStation", &[100, 100, 100, 80, 0]).unwrap();
        assert_eq!(garrison.radio_station, Some(80));
        assert_eq!(garrison.fortifications, None);
    }

    #[test]
    fn test_parse_tech_fort_base_full_eleven_slots() {
        let tech = parse_tech("FortBaseT2", &[100, 100, 70, 100, 100, 0, 0, 0, 0, 0, 0]).unwrap();
        assert_eq!(tech.t1_garrison, Some(100));
        assert_eq!(tech.t2_garrison, Some(100));
        assert_eq!(tech.advanced_bunkers, Some(0));
    }

    #[test]
    fn test_parse_tech_serializes_in_tree_order() {
        let tech = parse_tech("TownBase1", &[100, 18, 0, 10, 0, 0]).unwrap();
        let json = serde_json::to_string(&tech).unwrap();
        assert_eq!(
            json,
            r#"{"ProvisionalGarrison":100,"SmallGarrison":18,"LargeGarrison":0,"Industry":10,"OccupiedTown":0,"Fortifications":0}"#
        );
    }

    #[test]
    fn test_parse_tech_none_for_non_tech_and_empty() {
        // Facilities/depots have no tech.
        assert!(parse_tech("Seaport", &[100, 100]).is_none());
        // A base with no values present yields None rather than an empty Tech.
        assert!(parse_tech("Keep", &[]).is_none());
        // An unrecognized CodeName has no known tech layout.
        assert!(parse_tech("BrandNewBase", &[100, 100]).is_none());
    }

    #[test]
    fn test_stockpile_type_serialization() {
        let stockpile = Stockpile {
            name: String::new(),
            stockpile_type: "Seaport".to_string(),
            faction: Faction::Warden,
            hex: None,
            coords: None,
            is_reserve: false,
            items: vec![],
            tech: None,
            timestamp: None,
            shard: None,
            ingame_timestamp: None,
            resolution: None,
            errors: None,
        };
        let json = serde_json::to_string(&stockpile).unwrap();
        assert!(json.contains(r#""type":"Seaport""#));
    }

    #[test]
    fn test_stockpile_to_key() {
        let stockpile = Stockpile {
            name: "Test".to_string(),
            stockpile_type: "Seaport".to_string(),
            faction: Faction::Warden,
            hex: Some("Westgate".to_string()),
            coords: Some(StockpileCoords { x: 0.5, y: 0.5 }),
            is_reserve: false,
            items: vec![],
            tech: None,
            timestamp: Some(Utc::now()),
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
