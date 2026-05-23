//! File watcher for monitoring .sav file changes.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};

use crate::error::{FsSavError, Result};
use crate::models::ParseResult;
use crate::parser::parse_save;

/// Handle to a running file watcher.
pub struct WatchHandle {
    stop_flag: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
}

impl WatchHandle {
    /// Stop the watcher.
    pub fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    /// Check if the watcher is still running.
    pub fn is_alive(&self) -> bool {
        self.thread_handle
            .as_ref()
            .map(|h| !h.is_finished())
            .unwrap_or(false)
    }
}

impl Drop for WatchHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Callback type for watch results.
pub type WatchCallback = Box<dyn Fn(ParseResult) + Send + 'static>;

/// Watch a save file for changes.
///
/// # Arguments
///
/// * `path` - Path to the .sav file to watch
/// * `callback` - Function to call when changes are detected
/// * `poll_interval` - How often to check for changes (in seconds)
/// * `emit_only_changes` - If true, only emit when content actually changes
///
/// # Returns
///
/// A `WatchHandle` that can be used to stop watching.
pub fn watch_save<P, F>(
    path: P,
    callback: F,
    poll_interval: f64,
    emit_only_changes: bool,
) -> Result<WatchHandle>
where
    P: AsRef<Path>,
    F: Fn(ParseResult) + Send + 'static,
{
    let path = path.as_ref().to_path_buf();

    if !path.exists() {
        return Err(FsSavError::FileNotFound(path.display().to_string()));
    }

    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_clone = stop_flag.clone();

    // Track last parse result for change detection
    let mut last_result_json: Option<String> = None;

    // Initial parse
    if let Ok(result) = parse_save(&path) {
        if emit_only_changes {
            last_result_json = serde_json::to_string(&result).ok();
        }
        callback(result);
    }

    let thread_handle = thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel();

        // Create debounced watcher
        let debounce_duration = Duration::from_millis((poll_interval * 1000.0) as u64);
        let mut debouncer = match new_debouncer(debounce_duration, tx) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Failed to create watcher: {}", e);
                return;
            }
        };

        // Watch the file's parent directory (some systems don't support watching files directly)
        let watch_path = path.parent().unwrap_or(&path);
        if let Err(e) = debouncer
            .watcher()
            .watch(watch_path, RecursiveMode::NonRecursive)
        {
            eprintln!("Failed to watch path: {}", e);
            return;
        }

        let file_name = path.file_name();

        while !stop_flag_clone.load(Ordering::SeqCst) {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(Ok(events)) => {
                    // Check if any event is for our file
                    let is_our_file = events.iter().any(|event| {
                        event.path.file_name() == file_name
                            && matches!(event.kind, DebouncedEventKind::Any)
                    });

                    if is_our_file {
                        match parse_save(&path) {
                            Ok(result) => {
                                let should_emit = if emit_only_changes {
                                    let current_json = serde_json::to_string(&result).ok();
                                    let changed = current_json != last_result_json;
                                    last_result_json = current_json;
                                    changed
                                } else {
                                    true
                                };

                                if should_emit {
                                    callback(result);
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to parse save: {}", e);
                            }
                        }
                    }
                }
                Ok(Err(e)) => {
                    eprintln!("Watch error: {:?}", e);
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // Continue checking stop flag
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    break;
                }
            }
        }
    });

    Ok(WatchHandle {
        stop_flag,
        thread_handle: Some(thread_handle),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_watch_handle_stop() {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_flag_clone = stop_flag.clone();

        let thread_handle = thread::spawn(move || {
            while !stop_flag_clone.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(10));
            }
        });

        let mut handle = WatchHandle {
            stop_flag,
            thread_handle: Some(thread_handle),
        };

        assert!(handle.is_alive());
        handle.stop();
        assert!(!handle.is_alive());
    }
}
