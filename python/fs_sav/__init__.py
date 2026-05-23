"""
fs-sav - Foxhole save file parser.

Extracts stockpile data from Foxhole .sav files.

Example:
    >>> import fs_sav
    >>> stockpiles = fs_sav.parse_save("path/to/save.sav")
    >>> print(f"Found {len(stockpiles)} stockpiles")
    >>>
    >>> # With filters
    >>> seaports = fs_sav.parse_save("path/to/save.sav", stockpile_type="Seaport")
    >>> reserves = fs_sav.parse_save("path/to/save.sav", reserves=True)
    >>> terminus = fs_sav.parse_save("path/to/save.sav", hex="TerminusHex", with_items=True)
"""

from .fs_sav import info, parse_save, parse_save_bytes

__all__ = ["parse_save", "parse_save_bytes", "info"]

__version__ = "0.1.0"
