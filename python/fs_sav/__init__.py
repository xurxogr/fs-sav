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

import sys
from importlib.metadata import version

from .fs_sav import cli_main, info, parse_save, parse_save_bytes

__all__ = ["parse_save", "parse_save_bytes", "info", "main"]

__version__ = version("fs-sav")


def main() -> None:
    """Entry point for the ``fs-sav`` console script.

    Forwards ``sys.argv`` to the Rust CLI, so the command exposes the exact
    same subcommands and flags as the native binary.
    """
    raise SystemExit(cli_main(sys.argv))
