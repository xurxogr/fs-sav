//! fs-sav CLI logic.
//!
//! Shared by the native `fs-sav` binary (`src/main.rs`) and the Python
//! console-script entry point (`fs_sav:main`), so both expose identical
//! commands and parameters.

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use clap::{Args, Parser, Subcommand};
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};

use crate::models::Faction;
use crate::{parse_save, parse_save_bytes, Stockpile};

/// Parse a `--faction` value, accepting C/Colonial and W/Warden in any case.
pub(crate) fn parse_faction(s: &str) -> Result<Faction, String> {
    match s.to_ascii_lowercase().as_str() {
        "c" | "colonial" => Ok(Faction::Colonial),
        "w" | "warden" => Ok(Faction::Warden),
        other => Err(format!(
            "invalid faction '{other}' (expected C/Colonial or W/Warden)"
        )),
    }
}

#[derive(Parser)]
#[command(
    name = "fs-sav",
    author,
    version,
    about = "Foxhole save file parser - extracts stockpile data"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Filter options for stockpiles (stable properties)
#[derive(Args, Clone, Default)]
struct FilterArgs {
    /// Only public stockpiles (non-reserve)
    #[arg(long, conflicts_with = "reserves")]
    public: bool,

    /// Only reserve stockpiles
    #[arg(long, conflicts_with = "public")]
    reserves: bool,

    /// Filter by hex name (e.g., TerminusHex)
    #[arg(long)]
    hex: Option<String>,

    /// Filter by stockpile type (e.g., Seaport, StorageFacility)
    #[arg(long = "type")]
    stockpile_type: Option<String>,

    /// Filter by faction: C/Colonial or W/Warden (case-insensitive)
    #[arg(long, value_parser = parse_faction)]
    faction: Option<Faction>,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse a .sav file and output stockpiles as JSON (reads from stdin if no file given)
    Parse {
        /// Path to the .sav file (omit to read from stdin)
        file: Option<PathBuf>,

        /// Output compact JSON (no pretty printing)
        #[arg(short, long)]
        compact: bool,

        /// Only stockpiles with items
        #[arg(long)]
        with_items: bool,

        #[command(flatten)]
        filters: FilterArgs,
    },

    /// Watch a .sav file for changes and output NDJSON
    Watch {
        /// Path to the .sav file
        #[arg(required = true)]
        file: PathBuf,

        /// Poll interval in seconds
        #[arg(short, long, default_value = "1.0")]
        poll: f64,

        /// Only output stockpiles that changed (any field)
        #[arg(long, conflicts_with = "diff_items")]
        diff: bool,

        /// Only output stockpiles where items changed
        #[arg(long, conflicts_with = "diff")]
        diff_items: bool,

        #[command(flatten)]
        filters: FilterArgs,
    },

    /// Print version information
    Version,
}

/// Apply filters to a list of stockpiles
fn apply_filters(
    stockpiles: Vec<Stockpile>,
    filters: &FilterArgs,
    with_items: bool,
) -> Vec<Stockpile> {
    stockpiles
        .into_iter()
        .filter(|s| {
            // Public/reserve filter
            if filters.public && s.is_reserve {
                return false;
            }
            if filters.reserves && !s.is_reserve {
                return false;
            }

            // Faction filter
            if let Some(faction) = filters.faction {
                if s.faction != faction {
                    return false;
                }
            }

            // Hex filter
            if let Some(hex) = &filters.hex {
                if s.hex.as_ref() != Some(hex) {
                    return false;
                }
            }

            // Type filter
            if let Some(type_filter) = &filters.stockpile_type {
                if !s.stockpile_type.eq_ignore_ascii_case(type_filter) {
                    return false;
                }
            }

            // With items filter (only for parse command)
            if with_items && s.items.is_empty() {
                return false;
            }

            true
        })
        .collect()
}

/// Run the CLI with the given arguments (including the program name at index 0).
///
/// This mirrors `Cli::parse()` but takes an explicit argument vector so the
/// Python entry point can forward `sys.argv`.
pub fn run<I, T>(args: I) -> anyhow::Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = Cli::parse_from(args);

    match cli.command {
        Commands::Parse {
            file,
            compact,
            with_items,
            filters,
        } => {
            let stockpiles = match file {
                Some(path) => parse_save(&path)?.stockpiles,
                None => {
                    let mut buffer = Vec::new();
                    io::stdin().read_to_end(&mut buffer)?;
                    parse_save_bytes(&buffer)?
                }
            };

            let stockpiles = apply_filters(stockpiles, &filters, with_items);

            let output = if compact {
                serde_json::to_string(&stockpiles)?
            } else {
                serde_json::to_string_pretty(&stockpiles)?
            };

            println!("{}", output);
        }

        Commands::Watch {
            file,
            poll,
            diff,
            diff_items,
            filters,
        } => {
            let running = Arc::new(AtomicBool::new(true));
            let r = running.clone();

            // Handle Ctrl+C
            ctrlc::set_handler(move || {
                r.store(false, Ordering::SeqCst);
            })
            .expect("Error setting Ctrl-C handler");

            // Track previous state for diff modes
            let mut prev_stockpiles: HashMap<String, Stockpile> = HashMap::new();

            // Initial parse
            if let Ok(result) = parse_save(&file) {
                let stockpiles = apply_filters(result.stockpiles, &filters, false);
                let json = serde_json::to_string(&stockpiles).unwrap_or_default();
                println!("{}", json);
                io::stdout().flush().ok();

                // Store initial state for diff modes
                if diff || diff_items {
                    for stockpile in stockpiles {
                        prev_stockpiles.insert(stockpile.to_key(), stockpile);
                    }
                }
            }

            // Set up file watcher
            let (tx, rx) = std::sync::mpsc::channel();
            let debounce_duration = Duration::from_millis((poll * 1000.0) as u64);
            let mut debouncer =
                new_debouncer(debounce_duration, tx).expect("Failed to create watcher");

            let watch_path = file.parent().unwrap_or(&file);
            debouncer
                .watcher()
                .watch(watch_path, RecursiveMode::NonRecursive)
                .expect("Failed to watch path");

            let file_name = file.file_name();

            while running.load(Ordering::SeqCst) {
                match rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(Ok(events)) => {
                        let is_our_file = events.iter().any(|event| {
                            event.path.file_name() == file_name
                                && matches!(event.kind, DebouncedEventKind::Any)
                        });

                        if is_our_file {
                            if let Ok(result) = parse_save(&file) {
                                let stockpiles = apply_filters(result.stockpiles, &filters, false);

                                let output = if diff || diff_items {
                                    // Find changed stockpiles
                                    let mut changed: Vec<Stockpile> = Vec::new();

                                    for stockpile in &stockpiles {
                                        let key = stockpile.to_key();

                                        if let Some(prev) = prev_stockpiles.get(&key) {
                                            let is_changed = if diff_items {
                                                // Only compare items
                                                stockpile.items != prev.items
                                            } else {
                                                // Compare all fields (using JSON comparison)
                                                serde_json::to_string(stockpile).ok()
                                                    != serde_json::to_string(prev).ok()
                                            };

                                            if is_changed {
                                                changed.push(stockpile.clone());
                                            }
                                        } else {
                                            // New stockpile
                                            changed.push(stockpile.clone());
                                        }
                                    }

                                    // Update prev state
                                    prev_stockpiles.clear();
                                    for stockpile in stockpiles {
                                        prev_stockpiles.insert(stockpile.to_key(), stockpile);
                                    }

                                    if changed.is_empty() {
                                        None
                                    } else {
                                        Some(serde_json::to_string(&changed).unwrap_or_default())
                                    }
                                } else {
                                    // No diff mode - output all
                                    Some(serde_json::to_string(&stockpiles).unwrap_or_default())
                                };

                                if let Some(json) = output {
                                    println!("{}", json);
                                    io::stdout().flush().ok();
                                }
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        eprintln!("Watch error: {:?}", e);
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        }

        Commands::Version => {
            println!("fs-sav {}", env!("CARGO_PKG_VERSION"));
        }
    }

    Ok(())
}
