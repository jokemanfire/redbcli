use std::collections::HashMap;
use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
pub mod dynread;
pub mod flags;
pub mod pretty_print;
pub mod redbcontrol;

macro_rules! write_io {
    ($func_name:ident, $prefix:literal) => {
        pub fn $func_name(data: String) -> Result<(), String> {
            let out_data = format!("{}-> {} \n", $prefix, data);
            write!(std::io::stdout(), "{}", out_data).map_err(|e| e.to_string())?;
            std::io::stdout().flush().map_err(|e| e.to_string())?;
            Ok(())
        }
    };
}

write_io!(write_io_error, "error:");
write_io!(write_io_success, "success:");
write_io!(write_io_info, "info:");

#[derive(Debug, Default)]
pub struct TableMeta {
    pub name: String,
    pub key_type: String,
    pub value_type: String,
}

#[derive(Debug, Default)]
pub struct TableInfo {
    pub tables: Vec<TableMeta>,
}

#[derive(Debug, Default)]
pub struct KvInfo {
    pub kvdatas: Vec<(String, String)>,
    pub key_type: String,
    pub value_type: String,
}

pub type StrKvData = HashMap<String, String>;

/// Serializes table rows (formatted key/value cells) into a deterministic
/// pretty-printed JSON object, ready for editing in vim.
pub fn rows_to_json(rows: &[(String, String)]) -> Result<String, String> {
    let mut map = serde_json::Map::new();
    for (k, v) in rows {
        map.insert(k.clone(), serde_json::Value::String(v.clone()));
    }
    serde_json::to_string_pretty(&serde_json::Value::Object(map)).map_err(|e| e.to_string())
}

/// Parses edited JSON back into rows. Keys are the formatted cell strings;
/// values must be JSON strings.
pub fn json_to_rows(json: &str) -> Result<Vec<(String, String)>, String> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("invalid json: {e}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| "edited data must be a json object".to_string())?;
    let mut rows = Vec::with_capacity(object.len());
    for (k, v) in object {
        let text = v
            .as_str()
            .ok_or_else(|| format!("value for key '{k}' must be a json string"))?;
        rows.push((k.clone(), text.to_string()));
    }
    Ok(rows)
}

/// Directory that holds the REPL history file.
///
/// Order, first hit wins:
///
/// 1. `REDBCLI_HISTORY_DIR` (non-empty), for containers and tests;
/// 2. `$XDG_STATE_HOME/redbcli`, but only when `XDG_STATE_HOME` is absolute —
///    the XDG spec says a relative value must be ignored;
/// 3. `$HOME/.local/state/redbcli`, the spec's default for state files;
/// 4. `<temp>/redbcli`, the pre-0.4 location, kept only for environments with
///    neither a state home nor a home directory.
///
/// History is state, not scratch data: the old fixed `/tmp/redbcli` was shared
/// by every user on the host and stored database paths and edited content in a
/// world-readable directory.
pub fn history_dir() -> PathBuf {
    history_dir_from(
        std::env::var_os("REDBCLI_HISTORY_DIR"),
        std::env::var_os("XDG_STATE_HOME"),
        std::env::var_os("HOME"),
        &std::env::temp_dir(),
    )
}

/// Pure resolver behind [`history_dir`], so the precedence is testable without
/// mutating the process environment (tests run in parallel).
pub fn history_dir_from(
    override_dir: Option<OsString>,
    xdg_state_home: Option<OsString>,
    home: Option<OsString>,
    temp_dir: &Path,
) -> PathBuf {
    if let Some(dir) = override_dir.filter(|value| !value.is_empty()) {
        return PathBuf::from(dir);
    }
    if let Some(xdg) = xdg_state_home.filter(|value| !value.is_empty()) {
        let path = PathBuf::from(&xdg);
        if path.is_absolute() {
            return path.join("redbcli");
        }
    }
    if let Some(home) = home.filter(|value| !value.is_empty()) {
        return PathBuf::from(home)
            .join(".local")
            .join("state")
            .join("redbcli");
    }
    temp_dir.join("redbcli")
}

/// Create the history directory and file, returning `None` instead of panicking.
///
/// A REPL with no history is a degraded but usable CLI; a panic in `main` before
/// the first prompt means the tool cannot be used at all on a read-only or
/// foreign-owned `$HOME`, `/tmp`, or `XDG_STATE_HOME`.
pub fn prepare_history_file(dir: &Path) -> Option<PathBuf> {
    if let Err(err) = std::fs::create_dir_all(dir) {
        eprintln!(
            "warning: cannot create history directory {}: {err}",
            dir.display()
        );
        return None;
    }
    let file = dir.join("history.txt");
    if !file.exists() {
        if let Err(err) = std::fs::File::create(&file) {
            eprintln!(
                "warning: cannot create history file {}: {err}",
                file.display()
            );
            return None;
        }
    }
    Some(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_roundtrip_is_deterministic() {
        let rows = vec![
            ("zebra".to_string(), "2".to_string()),
            ("apple".to_string(), "1".to_string()),
        ];
        let json = rows_to_json(&rows).unwrap();
        assert_eq!(json, "{\n  \"apple\": \"1\",\n  \"zebra\": \"2\"\n}");
        let parsed = json_to_rows(&json).unwrap();
        assert_eq!(
            parsed,
            vec![
                ("apple".to_string(), "1".to_string()),
                ("zebra".to_string(), "2".to_string())
            ]
        );
    }

    #[test]
    fn json_rejects_non_object() {
        assert!(json_to_rows("[1, 2]").is_err());
        assert!(json_to_rows("\"str\"").is_err());
    }

    #[test]
    fn json_rejects_non_string_values() {
        let err = json_to_rows("{\"a\": 42}").unwrap_err();
        assert!(err.contains("must be a json string"), "got: {err}");
    }

    #[test]
    fn history_dir_prefers_the_explicit_override() {
        let dir = history_dir_from(
            Some(OsString::from("/srv/state/cli")),
            Some(OsString::from("/xdg/state")),
            Some(OsString::from("/home/app")),
            Path::new("/tmp"),
        );
        assert_eq!(dir, PathBuf::from("/srv/state/cli"));
    }

    #[test]
    fn history_dir_uses_an_absolute_xdg_state_home() {
        let dir = history_dir_from(
            None,
            Some(OsString::from("/xdg/state")),
            Some(OsString::from("/home/app")),
            Path::new("/tmp"),
        );
        assert_eq!(dir, PathBuf::from("/xdg/state/redbcli"));
    }

    #[test]
    fn history_dir_ignores_a_relative_xdg_state_home() {
        // The XDG spec says a relative value is invalid and must be ignored.
        let dir = history_dir_from(
            None,
            Some(OsString::from("relative/state")),
            Some(OsString::from("/home/app")),
            Path::new("/tmp"),
        );
        assert_eq!(dir, PathBuf::from("/home/app/.local/state/redbcli"));
    }

    #[test]
    fn history_dir_ignores_empty_values() {
        let dir = history_dir_from(
            Some(OsString::new()),
            Some(OsString::new()),
            Some(OsString::new()),
            Path::new("/tmp"),
        );
        assert_eq!(dir, PathBuf::from("/tmp/redbcli"));
    }

    #[test]
    fn history_dir_falls_back_to_home_then_temp() {
        let with_home = history_dir_from(
            None,
            None,
            Some(OsString::from("/home/app")),
            Path::new("/tmp"),
        );
        assert_eq!(with_home, PathBuf::from("/home/app/.local/state/redbcli"));

        let without_home = history_dir_from(None, None, None, Path::new("/tmp"));
        assert_eq!(without_home, PathBuf::from("/tmp/redbcli"));
    }

    #[test]
    fn prepare_history_file_creates_the_directory_and_file() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("nested").join("redbcli");

        let file = prepare_history_file(&dir).expect("history file should be created");

        assert_eq!(file, dir.join("history.txt"));
        assert!(file.exists());
        // Second call reuses the existing file instead of truncating it.
        std::fs::write(&file, "set db.redb\n").unwrap();
        assert!(prepare_history_file(&dir).is_some());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "set db.redb\n");
    }

    #[test]
    fn prepare_history_file_reports_failure_instead_of_panicking() {
        let root = tempfile::tempdir().unwrap();
        let blocker = root.path().join("blocked");
        std::fs::write(&blocker, "not a directory").unwrap();

        assert!(prepare_history_file(&blocker.join("redbcli")).is_none());
    }

    #[test]
    fn json_preserves_tuple_and_hex_cells() {
        let rows = vec![
            ("(1, 2)".to_string(), "0x01abff".to_string()),
            ("(3, 4)".to_string(), "100".to_string()),
        ];
        let parsed = json_to_rows(&rows_to_json(&rows).unwrap()).unwrap();
        assert_eq!(parsed, rows);
    }
}
