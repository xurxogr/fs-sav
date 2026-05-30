"""Tests for fs_sav Python bindings."""

import fs_sav
import pytest

# Path to test fixture
TEST_SAV_PATH = "tests/fixtures/test.sav"


class TestParseSave:
    """Tests for parse_save function."""

    def test_parse_save_returns_list(self):
        """parse_save should return a list."""
        result = fs_sav.parse_save(TEST_SAV_PATH)
        assert isinstance(result, list)

    def test_parse_save_finds_stockpiles(self):
        """parse_save should find all stockpiles in test file."""
        stockpiles = fs_sav.parse_save(TEST_SAV_PATH)
        assert len(stockpiles) == 26

    def test_stockpile_has_required_fields(self):
        """Each stockpile should have required fields."""
        stockpiles = fs_sav.parse_save(TEST_SAV_PATH)
        required_fields = ["name", "type", "hex", "coords", "is_reserve", "items", "timestamp"]

        for stockpile in stockpiles:
            for field in required_fields:
                assert field in stockpile, f"Missing field: {field}"

    def test_stockpile_coords_structure(self):
        """Stockpile coords should have x and y."""
        stockpiles = fs_sav.parse_save(TEST_SAV_PATH)

        for stockpile in stockpiles:
            if stockpile["coords"] is not None:
                assert "x" in stockpile["coords"]
                assert "y" in stockpile["coords"]
                assert 0.0 <= stockpile["coords"]["x"] <= 1.0
                assert 0.0 <= stockpile["coords"]["y"] <= 1.0

    def test_parse_nonexistent_file_raises(self):
        """parse_save should raise on nonexistent file."""
        with pytest.raises(RuntimeError):
            fs_sav.parse_save("nonexistent.sav")


class TestFilters:
    """Tests for filter options."""

    def test_filter_public(self):
        """--public filter should return only non-reserve stockpiles."""
        stockpiles = fs_sav.parse_save(TEST_SAV_PATH, public=True)

        for stockpile in stockpiles:
            assert stockpile["is_reserve"] is False

    def test_filter_reserves(self):
        """--reserves filter should return only reserve stockpiles."""
        stockpiles = fs_sav.parse_save(TEST_SAV_PATH, reserves=True)

        for stockpile in stockpiles:
            assert stockpile["is_reserve"] is True

    def test_filter_hex(self):
        """--hex filter should return only stockpiles in that hex."""
        stockpiles = fs_sav.parse_save(TEST_SAV_PATH, hex="TerminusHex")

        assert len(stockpiles) > 0
        for stockpile in stockpiles:
            assert stockpile["hex"] == "TerminusHex"

    def test_filter_hex_not_found(self):
        """--hex filter with unknown hex should return empty list."""
        stockpiles = fs_sav.parse_save(TEST_SAV_PATH, hex="NonexistentHex")
        assert len(stockpiles) == 0

    def test_filter_type(self):
        """--stockpile_type filter should return only that type."""
        stockpiles = fs_sav.parse_save(TEST_SAV_PATH, stockpile_type="Seaport")

        assert len(stockpiles) == 1
        assert stockpiles[0]["type"] == "Seaport"

    def test_filter_type_case_insensitive(self):
        """--stockpile_type filter should be case insensitive."""
        stockpiles_lower = fs_sav.parse_save(TEST_SAV_PATH, stockpile_type="seaport")
        stockpiles_upper = fs_sav.parse_save(TEST_SAV_PATH, stockpile_type="SEAPORT")

        assert len(stockpiles_lower) == len(stockpiles_upper) == 1

    def test_filter_faction(self):
        """faction filter should return only stockpiles of that faction."""
        for value in ("Warden", "Colonial"):
            stockpiles = fs_sav.parse_save(TEST_SAV_PATH, faction=value)
            for stockpile in stockpiles:
                assert stockpile["faction"] == value

    def test_filter_faction_case_insensitive_and_short(self):
        """faction filter should accept short codes and any case."""
        wardens_long = fs_sav.parse_save(TEST_SAV_PATH, faction="warden")
        wardens_short = fs_sav.parse_save(TEST_SAV_PATH, faction="W")
        assert len(wardens_long) == len(wardens_short)

    def test_filter_faction_invalid_raises(self):
        """An invalid faction value should raise ValueError."""
        with pytest.raises(ValueError):
            fs_sav.parse_save(TEST_SAV_PATH, faction="Neutral")

    def test_filter_with_items(self):
        """--with_items filter should return only stockpiles with items."""
        stockpiles = fs_sav.parse_save(TEST_SAV_PATH, with_items=True)

        assert len(stockpiles) > 0
        for stockpile in stockpiles:
            assert len(stockpile["items"]) > 0

    def test_filter_combined(self):
        """Multiple filters should work together."""
        stockpiles = fs_sav.parse_save(TEST_SAV_PATH, hex="TerminusHex", public=True)

        for stockpile in stockpiles:
            assert stockpile["hex"] == "TerminusHex"
            assert stockpile["is_reserve"] is False


class TestParseSaveBytes:
    """Tests for parse_save_bytes function."""

    def test_parse_save_bytes(self):
        """parse_save_bytes should parse raw bytes."""
        with open(TEST_SAV_PATH, "rb") as f:
            data = f.read()

        stockpiles = fs_sav.parse_save_bytes(data)
        assert len(stockpiles) == 26

    def test_parse_save_bytes_with_filters(self):
        """parse_save_bytes should support filters."""
        with open(TEST_SAV_PATH, "rb") as f:
            data = f.read()

        stockpiles = fs_sav.parse_save_bytes(data, stockpile_type="Seaport")
        assert len(stockpiles) == 1

    def test_parse_save_bytes_invalid_data(self):
        """parse_save_bytes should raise on invalid data."""
        with pytest.raises(RuntimeError):
            fs_sav.parse_save_bytes(b"invalid data")


class TestInfo:
    """Tests for info function."""

    def test_info_returns_dict(self):
        """info should return a dictionary."""
        result = fs_sav.info()
        assert isinstance(result, dict)

    def test_info_has_implementation(self):
        """info should have implementation field."""
        result = fs_sav.info()
        assert result["implementation"] == "rust"

    def test_info_has_version(self):
        """info should have version field."""
        result = fs_sav.info()
        assert "version" in result
        assert isinstance(result["version"], str)


class TestStockpileTypes:
    """Tests for stockpile type coverage."""

    def test_all_types_present(self):
        """Test file should contain all stockpile types."""
        stockpiles = fs_sav.parse_save(TEST_SAV_PATH)
        types = {s["type"] for s in stockpiles}

        expected_types = {
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
        }

        assert types == expected_types
