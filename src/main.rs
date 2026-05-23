//! fs-sav CLI - Foxhole save file parser.
//!
//! Thin wrapper around [`fs_sav::cli::run`], which holds the actual command
//! definitions so the native binary and the Python console-script stay in sync.

fn main() -> anyhow::Result<()> {
    fs_sav::cli::run(std::env::args_os())
}
