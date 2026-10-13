//! Locate the Foxhole map save files (`<SteamID64>_MapData.sav`) on this
//! machine.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Steam app id of Foxhole.
const FOXHOLE_APP_ID: &str = "505460";

/// Suffix of the map save file name: `<SteamID64>_MapData.sav`.
const MAP_SAVE_SUFFIX: &str = "_MapData.sav";

/// Steam installation roots, relative to the home directory.
const STEAM_ROOTS: [&str; 5] = [
    ".steam/steam",
    ".steam/root",
    ".steam/debian-installation",
    ".local/share/Steam",
    ".var/app/com.valvesoftware.Steam/.local/share/Steam",
];

/// A Foxhole map save file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveFile {
    /// Path to the save file
    pub path: PathBuf,
    /// Steam account the save belongs to, from the file name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steam_id: Option<String>,
    /// When the game last wrote it
    pub modified: DateTime<Utc>,
    /// File size in bytes
    pub size_bytes: u64,
}

/// Find the Foxhole map saves on this machine, newest first.
///
/// Looks in the Windows save folder (`%LOCALAPPDATA%\Foxhole\Saved\SaveGames`),
/// the Windows user folders seen from WSL, and the Proton prefix of every
/// Steam library on Linux (native and Flatpak Steam, including extra libraries
/// listed in `libraryfolders.vdf`).
pub fn find_save_files() -> Vec<SaveFile> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let local_app_data = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let wsl_users = Some(PathBuf::from("/mnt/c/Users"));
    find_save_files_in(&save_dirs(
        home.as_deref(),
        local_app_data.as_deref(),
        wsl_users.as_deref(),
    ))
}

/// Folders where the game may keep its saves.
fn save_dirs(
    home: Option<&Path>,
    local_app_data: Option<&Path>,
    wsl_users: Option<&Path>,
) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(local_app_data) = local_app_data {
        dirs.push(windows_save_dir(local_app_data));
    }

    if let Some(users) = wsl_users.and_then(|users| std::fs::read_dir(users).ok()) {
        for user in users.flatten() {
            dirs.push(windows_save_dir(&user.path().join("AppData").join("Local")));
        }
    }

    if let Some(home) = home {
        for library in steam_libraries(home) {
            dirs.push(
                library
                    .join("steamapps/compatdata")
                    .join(FOXHOLE_APP_ID)
                    .join("pfx/drive_c/users/steamuser/AppData/Local/Foxhole/Saved/SaveGames"),
            );
        }
    }

    dirs
}

/// `Foxhole\Saved\SaveGames` under a Windows local application data folder.
fn windows_save_dir(local_app_data: &Path) -> PathBuf {
    local_app_data
        .join("Foxhole")
        .join("Saved")
        .join("SaveGames")
}

/// Steam roots under the home directory, plus the extra libraries listed in
/// their `libraryfolders.vdf`.
fn steam_libraries(home: &Path) -> Vec<PathBuf> {
    let mut libraries = Vec::new();
    for root in STEAM_ROOTS.iter().map(|root| home.join(root)) {
        if !root.is_dir() {
            continue;
        }
        if let Ok(vdf) = std::fs::read_to_string(root.join("steamapps/libraryfolders.vdf")) {
            libraries.extend(library_paths(&vdf));
        }
        libraries.push(root);
    }
    libraries
}

/// Library paths of a `libraryfolders.vdf` file (its `"path"` entries).
fn library_paths(vdf: &str) -> Vec<PathBuf> {
    vdf.lines()
        .filter_map(|line| {
            let mut quoted = line.split('"').skip(1).step_by(2);
            match (quoted.next(), quoted.next()) {
                (Some("path"), Some(path)) => Some(PathBuf::from(path.replace("\\\\", "\\"))),
                _ => None,
            }
        })
        .collect()
}

/// Map saves in the given folders, without duplicates, newest first.
fn find_save_files_in(dirs: &[PathBuf]) -> Vec<SaveFile> {
    let mut seen = HashSet::new();
    let mut saves = Vec::new();

    for entry in dirs
        .iter()
        .filter_map(|dir| std::fs::read_dir(dir).ok())
        .flat_map(|entries| entries.flatten())
    {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(prefix) = file_name.strip_suffix(MAP_SAVE_SUFFIX) else {
            continue;
        };
        let steam_id = (!prefix.is_empty() && prefix.bytes().all(|b| b.is_ascii_digit()))
            .then(|| prefix.to_string());
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        // Several Steam roots are usually links to the same installation.
        let real_path = std::fs::canonicalize(&path).unwrap_or(path);
        if !seen.insert(real_path.clone()) {
            continue;
        }
        let Ok(modified) = metadata.modified() else {
            continue;
        };

        saves.push(SaveFile {
            path: real_path,
            steam_id,
            modified: DateTime::<Utc>::from(modified),
            size_bytes: metadata.len(),
        });
    }

    saves.sort_by_key(|save| std::cmp::Reverse(save.modified));
    saves
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;
    use std::time::{Duration, SystemTime};

    fn touch(path: &Path, age: Duration) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"save").unwrap();
        let file = fs::File::options().write(true).open(path).unwrap();
        file.set_modified(SystemTime::now() - age).unwrap();
    }

    fn proton_save_dir(library: &Path) -> PathBuf {
        library.join(
            "steamapps/compatdata/505460/pfx/drive_c/users/steamuser/AppData/Local/Foxhole/Saved/SaveGames",
        )
    }

    #[test]
    fn test_library_paths() {
        let vdf = r#"
"libraryfolders"
{
	"0"
	{
		"path"		"/home/user/.local/share/Steam"
		"label"		""
	}
	"1"
	{
		"path"		"D:\\SteamLibrary"
	}
}
"#;
        assert_eq!(
            library_paths(vdf),
            [
                PathBuf::from("/home/user/.local/share/Steam"),
                PathBuf::from("D:\\SteamLibrary"),
            ]
        );
    }

    #[test]
    fn test_finds_saves_in_every_location_newest_first() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let local_app_data = tmp.path().join("local");
        let wsl_users = tmp.path().join("users");
        let extra_library = tmp.path().join("games");

        // Windows save folder
        touch(
            &windows_save_dir(&local_app_data).join("111_MapData.sav"),
            Duration::from_secs(300),
        );
        // A Windows user folder seen from WSL
        touch(
            &windows_save_dir(&wsl_users.join("someone/AppData/Local")).join("222_MapData.sav"),
            Duration::from_secs(100),
        );
        // Flatpak Steam, whose libraryfolders.vdf lists an extra library
        let flatpak = home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam");
        fs::create_dir_all(flatpak.join("steamapps")).unwrap();
        fs::write(
            flatpak.join("steamapps/libraryfolders.vdf"),
            format!("\"path\"\t\t\"{}\"\n", extra_library.display()),
        )
        .unwrap();
        touch(
            &proton_save_dir(&extra_library).join("333_MapData.sav"),
            Duration::from_secs(10),
        );
        // Files that are not map saves are ignored
        touch(
            &windows_save_dir(&local_app_data).join("111.sav"),
            Duration::from_secs(0),
        );

        let saves = find_save_files_in(&save_dirs(
            Some(&home),
            Some(&local_app_data),
            Some(&wsl_users),
        ));

        let ids: Vec<_> = saves.iter().map(|s| s.steam_id.as_deref()).collect();
        assert_eq!(ids, [Some("333"), Some("222"), Some("111")]);
        assert!(saves.iter().all(|s| s.size_bytes == 4));
    }

    #[cfg(unix)]
    #[test]
    fn test_linked_steam_roots_are_reported_once() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let steam = home.join(".local/share/Steam");
        touch(
            &proton_save_dir(&steam).join("444_MapData.sav"),
            Duration::from_secs(0),
        );
        fs::create_dir_all(home.join(".steam")).unwrap();
        std::os::unix::fs::symlink(&steam, home.join(".steam/steam")).unwrap();

        let saves = find_save_files_in(&save_dirs(Some(&home), None, None));
        assert_eq!(saves.len(), 1);
        assert_eq!(saves[0].steam_id.as_deref(), Some("444"));
    }

    #[test]
    fn test_no_saves() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(find_save_files_in(&save_dirs(Some(tmp.path()), None, None)).is_empty());
    }
}
