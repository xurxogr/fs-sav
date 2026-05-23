//! # fs-sav
//!
//! Foxhole save file parser - extracts stockpile data from .sav files.
//!
//! ## Features
//!
//! - Parse Foxhole .sav files using the uesave library
//! - Extract stockpile information (items, locations, types)
//! - Watch files for changes with debounced notifications
//! - Output in JSON format compatible with foxhole-stockpiles
//!
//! ## Example
//!
//! ```rust,no_run
//! use fs_sav::{parse_save, ParseResult};
//! use std::path::Path;
//!
//! fn main() -> fs_sav::Result<()> {
//!     let result = parse_save("path/to/save.sav")?;
//!     println!("Found {} stockpiles", result.stockpiles.len());
//!     Ok(())
//! }
//! ```

pub mod cli;
pub mod error;
pub mod models;
pub mod parser;
pub mod watcher;

// Re-export main types for convenience
pub use error::{FsSavError, Result};
pub use models::{
    ParseResult, ParserInfo, Stockpile, StockpileCoords, StockpileItem, StockpileType,
};
pub use parser::{parse_save, parse_save_bytes};
pub use watcher::{watch_save, WatchHandle};

/// Get parser information.
pub fn info() -> ParserInfo {
    ParserInfo::default()
}

// Python bindings (when compiled with --features python)
#[cfg(feature = "python")]
mod python {
    use pyo3::prelude::*;

    use crate::models::{ParserInfo, Stockpile};
    use crate::parser;

    /// Apply filters to stockpiles (mirrors CLI logic)
    fn apply_filters(
        stockpiles: Vec<Stockpile>,
        public: bool,
        reserves: bool,
        hex: Option<&str>,
        stockpile_type: Option<&str>,
        with_items: bool,
    ) -> Vec<Stockpile> {
        stockpiles
            .into_iter()
            .filter(|s| {
                // Public/reserve filter
                if public && s.is_reserve {
                    return false;
                }
                if reserves && !s.is_reserve {
                    return false;
                }

                // Hex filter
                if let Some(h) = hex {
                    if s.hex.as_deref() != Some(h) {
                        return false;
                    }
                }

                // Type filter
                if let Some(type_filter) = stockpile_type {
                    let type_str = serde_json::to_string(&s.stockpile_type)
                        .unwrap_or_default()
                        .trim_matches('"')
                        .to_string();
                    if !type_str.eq_ignore_ascii_case(type_filter) {
                        return false;
                    }
                }

                // With items filter
                if with_items && s.items.is_empty() {
                    return false;
                }

                true
            })
            .collect()
    }

    /// Convert stockpiles to Python list
    fn stockpiles_to_py(py: Python<'_>, stockpiles: Vec<Stockpile>) -> PyResult<Py<PyAny>> {
        let json_str = serde_json::to_string(&stockpiles)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        let json_module = PyModule::import(py, "json")?;
        let result = json_module.call_method1("loads", (json_str,))?;
        Ok(result.unbind())
    }

    /// Parse a .sav file and return the stockpiles as a Python list.
    ///
    /// Args:
    ///     path: Path to the .sav file
    ///     public: Only return public stockpiles (non-reserve)
    ///     reserves: Only return reserve stockpiles
    ///     hex: Filter by hex name (e.g., "TerminusHex")
    ///     stockpile_type: Filter by stockpile type (e.g., "Seaport")
    ///     with_items: Only return stockpiles with items
    ///
    /// Returns:
    ///     List of stockpile dictionaries
    #[pyfunction]
    #[pyo3(signature = (path, *, public=false, reserves=false, hex=None, stockpile_type=None, with_items=false))]
    fn parse_save(
        path: &str,
        public: bool,
        reserves: bool,
        hex: Option<&str>,
        stockpile_type: Option<&str>,
        with_items: bool,
    ) -> PyResult<Py<PyAny>> {
        let result = parser::parse_save(path)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        let stockpiles = apply_filters(
            result.stockpiles,
            public,
            reserves,
            hex,
            stockpile_type,
            with_items,
        );

        Python::with_gil(|py| stockpiles_to_py(py, stockpiles))
    }

    /// Parse .sav data from bytes and return the stockpiles as a Python list.
    ///
    /// Args:
    ///     data: Raw bytes of the .sav file
    ///     public: Only return public stockpiles (non-reserve)
    ///     reserves: Only return reserve stockpiles
    ///     hex: Filter by hex name (e.g., "TerminusHex")
    ///     stockpile_type: Filter by stockpile type (e.g., "Seaport")
    ///     with_items: Only return stockpiles with items
    ///
    /// Returns:
    ///     List of stockpile dictionaries
    #[pyfunction]
    #[pyo3(signature = (data, *, public=false, reserves=false, hex=None, stockpile_type=None, with_items=false))]
    fn parse_save_bytes(
        data: &[u8],
        public: bool,
        reserves: bool,
        hex: Option<&str>,
        stockpile_type: Option<&str>,
        with_items: bool,
    ) -> PyResult<Py<PyAny>> {
        let stockpiles = parser::parse_save_bytes(data)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        let stockpiles = apply_filters(
            stockpiles,
            public,
            reserves,
            hex,
            stockpile_type,
            with_items,
        );

        Python::with_gil(|py| stockpiles_to_py(py, stockpiles))
    }

    /// Get parser information.
    ///
    /// Returns:
    ///     Dictionary with 'implementation' and 'version' keys
    #[pyfunction]
    fn info() -> PyResult<Py<PyAny>> {
        let info = ParserInfo::default();

        Python::with_gil(|py| {
            let json_str = serde_json::to_string(&info)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

            let json_module = PyModule::import(py, "json")?;
            let result = json_module.call_method1("loads", (json_str,))?;
            Ok(result.unbind())
        })
    }

    /// Run the `fs-sav` command-line interface.
    ///
    /// Forwards `argv` (including the program name at index 0) to the exact
    /// same clap parser used by the native binary, so the Python console
    /// script accepts identical commands and parameters.
    ///
    /// Args:
    ///     argv: Argument vector, typically `sys.argv`.
    ///
    /// Returns:
    ///     Process exit code (0 on success, 1 on error).
    #[pyfunction]
    fn cli_main(py: Python<'_>, argv: Vec<String>) -> i32 {
        // Release the GIL: the CLI does blocking I/O and (for `watch`) runs
        // until interrupted.
        py.allow_threads(|| match crate::cli::run(argv) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("Error: {}", e);
                1
            }
        })
    }

    /// fs-sav Python module
    #[pymodule]
    fn fs_sav(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_function(wrap_pyfunction!(parse_save, m)?)?;
        m.add_function(wrap_pyfunction!(parse_save_bytes, m)?)?;
        m.add_function(wrap_pyfunction!(info, m)?)?;
        m.add_function(wrap_pyfunction!(cli_main, m)?)?;
        Ok(())
    }
}
