"""Tests for fs_sav Python bindings."""

import fs_sav
import pytest

# Path to test fixture
TEST_SAV_PATH = "tests/fixtures/test.sav"
# Fixture whose tooltips all have current details (used to exercise filters)
FILTER_SAV_PATH = "tests/fixtures/refinery_mapdata.sav"


class TestParseSave:
    """Tests for parse_save function."""

    def test_parse_save_returns_list(self):
        """parse_save should return a list."""
        result = fs_sav.parse_save(TEST_SAV_PATH)
        assert isinstance(result, list)

    def test_parse_save_finds_stockpiles(self):
        """parse_save should only return tooltips with current details."""
        stockpiles = fs_sav.parse_save(TEST_SAV_PATH)
        # 25 of the 26 pinned tooltips were last updated with their details
        # without details and are skipped.
        assert len(stockpiles) == 1

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
        stockpiles = fs_sav.parse_save(FILTER_SAV_PATH, public=True)

        for stockpile in stockpiles:
            assert stockpile["is_reserve"] is False

    def test_filter_reserves(self):
        """--reserves filter should return only reserve stockpiles."""
        stockpiles = fs_sav.parse_save(FILTER_SAV_PATH, reserves=True)

        for stockpile in stockpiles:
            assert stockpile["is_reserve"] is True

    def test_filter_hex(self):
        """--hex filter should return only stockpiles in that hex."""
        stockpiles = fs_sav.parse_save(FILTER_SAV_PATH, hex="TerminusHex")

        assert len(stockpiles) > 0
        for stockpile in stockpiles:
            assert stockpile["hex"] == "TerminusHex"

    def test_filter_hex_not_found(self):
        """--hex filter with unknown hex should return empty list."""
        stockpiles = fs_sav.parse_save(FILTER_SAV_PATH, hex="NonexistentHex")
        assert len(stockpiles) == 0

    def test_filter_type(self):
        """--stockpile_type filter should return only that type."""
        stockpiles = fs_sav.parse_save(FILTER_SAV_PATH, stockpile_type="Seaport")

        # 3 seaports, each with one reserve stockpile
        assert len(stockpiles) == 6
        assert all(s["type"] == "Seaport" for s in stockpiles)

    def test_filter_type_case_insensitive(self):
        """--stockpile_type filter should be case insensitive."""
        stockpiles_lower = fs_sav.parse_save(FILTER_SAV_PATH, stockpile_type="seaport")
        stockpiles_upper = fs_sav.parse_save(FILTER_SAV_PATH, stockpile_type="SEAPORT")

        assert len(stockpiles_lower) == len(stockpiles_upper) == 6

    def test_filter_faction(self):
        """faction filter should return only stockpiles of that faction."""
        for value in ("Warden", "Colonial"):
            stockpiles = fs_sav.parse_save(FILTER_SAV_PATH, faction=value)
            for stockpile in stockpiles:
                assert stockpile["faction"] == value

    def test_filter_faction_case_insensitive_and_short(self):
        """faction filter should accept short codes and any case."""
        wardens_long = fs_sav.parse_save(FILTER_SAV_PATH, faction="warden")
        wardens_short = fs_sav.parse_save(FILTER_SAV_PATH, faction="W")
        assert len(wardens_long) == len(wardens_short)

    def test_filter_faction_invalid_raises(self):
        """An invalid faction value should raise ValueError."""
        with pytest.raises(ValueError):
            fs_sav.parse_save(FILTER_SAV_PATH, faction="Neutral")

    def test_filter_with_items(self):
        """--with_items filter should return only stockpiles with items."""
        stockpiles = fs_sav.parse_save(FILTER_SAV_PATH, with_items=True)

        assert len(stockpiles) > 0
        for stockpile in stockpiles:
            assert len(stockpile["items"]) > 0

    def test_filter_combined(self):
        """Multiple filters should work together."""
        stockpiles = fs_sav.parse_save(FILTER_SAV_PATH, hex="TerminusHex", public=True)

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
        assert len(stockpiles) == 1

    def test_parse_save_bytes_with_filters(self):
        """parse_save_bytes should support filters."""
        with open(FILTER_SAV_PATH, "rb") as f:
            data = f.read()

        stockpiles = fs_sav.parse_save_bytes(data, stockpile_type="Seaport")
        assert len(stockpiles) == 6

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

    def test_unavailable_tooltips_skipped(self):
        """Tooltips whose latest details carry no data are not returned."""
        stockpiles = fs_sav.parse_save(TEST_SAV_PATH)
        types = {s["type"] for s in stockpiles}

        # Only TownBase3 has current details in the test file.
        assert types == {"TownBase3"}

    def test_types_from_current_tooltips(self):
        """Types are the raw in-game CodeNames of tooltips with current details."""
        stockpiles = fs_sav.parse_save(FILTER_SAV_PATH)
        types = {s["type"] for s in stockpiles}

        assert types == {"Seaport", "AircraftDepot", "Refinery"}


class TestRefinery:
    """Tests for refinery production data."""

    REFINERY_SAV_PATH = "tests/fixtures/refinery_mapdata.sav"

    def test_refinery_orders_and_storages(self):
        """The refinery stockpile carries the selected queue and stored output."""
        stockpiles = fs_sav.parse_save(self.REFINERY_SAV_PATH)

        refineries = [s for s in stockpiles if "refinery" in s]
        assert len(refineries) == 1

        refinery = refineries[0]["refinery"]
        assert refineries[0]["type"] == "Refinery"
        assert refinery["orders_access"] == {"level": "public"}
        assert refinery["orders"] == [
            {"slot": 0, "code": "Cloth", "source": 0, "refined": 50, "duration": 0}
        ]

        storages = refinery["storages"]
        assert len(storages) == 7
        assert all(s["access"] == {"level": "squad", "squad_id": 1} for s in storages)
        refined = {s["slot"]: s["refined"] for s in storages}
        assert refined[0] == 10
        assert refined[1] == 5309

    def test_other_stockpiles_have_no_refinery(self):
        """Only the refinery's main stockpile carries refinery data."""
        stockpiles = fs_sav.parse_save(self.REFINERY_SAV_PATH)
        assert all(
            "refinery" not in s for s in stockpiles if s["type"] != "Refinery" or s["is_reserve"]
        )


class TestWarnings:
    """Tests for warnings about skipped tooltips."""

    def test_parse_save_warns_for_skipped_tooltips(self):
        """Each skipped tooltip is reported as a UserWarning."""
        with pytest.warns(UserWarning) as record:
            stockpiles = fs_sav.parse_save(TEST_SAV_PATH)

        assert len(stockpiles) == 1
        messages = [str(w.message) for w in record]
        assert len(messages) == 25
        assert all("the game had no details for it" in m for m in messages)

    def test_parse_save_bytes_warns_for_skipped_tooltips(self):
        """parse_save_bytes reports the same warnings as parse_save."""
        with open(TEST_SAV_PATH, "rb") as f:
            data = f.read()

        with pytest.warns(UserWarning) as record:
            fs_sav.parse_save_bytes(data)

        assert len(record) == 25


class TestStockpileKey:
    """Tests for stockpile_key."""

    def test_key_format(self):
        """The key is type:hex:x,y:name with coords rounded to 4 decimals."""
        stockpile = {
            "name": "Logi",
            "type": "Seaport",
            "hex": "Westgate",
            "coords": {"x": 0.457745, "y": 0.664469},
        }
        assert fs_sav.stockpile_key(stockpile) == "Seaport:Westgate:0.4577,0.6645:Logi"

    def test_key_defaults(self):
        """Missing fields use the documented defaults."""
        assert fs_sav.stockpile_key({}) == "Undefined::0,0:"
        assert fs_sav.stockpile_key({"type": "Seaport", "hex": None, "coords": None}) == (
            "Seaport::0,0:"
        )

    def test_keys_unique_for_parsed_stockpiles(self):
        """Every parsed stockpile gets a distinct key."""
        stockpiles = fs_sav.parse_save(FILTER_SAV_PATH)
        keys = {fs_sav.stockpile_key(s) for s in stockpiles}
        assert len(keys) == len(stockpiles)

    def test_invalid_field_type_raises(self):
        """A field with the wrong type raises TypeError."""
        with pytest.raises(TypeError):
            fs_sav.stockpile_key({"type": 1})
        with pytest.raises(TypeError):
            fs_sav.stockpile_key({"coords": "0,0"})
