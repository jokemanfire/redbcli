use std::collections::HashMap;
use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
pub mod dynread;
pub mod flags;
pub mod pretty_print;
pub mod redbcontrol;

/// Errors surfaced by the command handlers.
///
/// `redb::Error::Corrupted` renders as `DB corrupted: ...`, which is reserved
/// for damaged database files. A table whose `(K, V)` pair is outside this
/// build's type registry is a limitation of the binary, not damage to the
/// file, so it is carried by its own variant and rendered without the
/// corruption prefix.
#[derive(Debug)]
pub enum CliError {
    /// Any redb-level error, rendered verbatim by redb.
    Redb(redb::Error),
    /// The table's key/value types are not covered by this build's registry.
    UnsupportedTableType { detail: String },
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::Redb(e) => e.fmt(f),
            CliError::UnsupportedTableType { detail } => write!(
                f,
                "unsupported table type: {detail} (the database file is not damaged; \
                 this build of redbcli simply cannot read or write this type yet)"
            ),
        }
    }
}

impl std::error::Error for CliError {}

impl From<redb::Error> for CliError {
    fn from(e: redb::Error) -> Self {
        CliError::Redb(e)
    }
}

/// redb splits its failures across several error enums; every one of them
/// converts into `redb::Error`, which keeps the original rendering.
macro_rules! cli_error_from_redb {
    ($($ty:ty),* $(,)?) => {
        $(
            impl From<$ty> for CliError {
                fn from(e: $ty) -> Self {
                    CliError::Redb(e.into())
                }
            }
        )*
    };
}

cli_error_from_redb!(
    redb::TableError,
    redb::StorageError,
    redb::TransactionError,
    redb::CommitError,
    redb::DatabaseError,
    redb::SavepointError,
);

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
/// 3. `$HOME/.local/state/redbcli`, the spec's default for state files.
///
/// There is deliberately no `<temp>/redbcli` fallback. History is state, not
/// scratch data — it records database paths and edited content — and a
/// predictable name under a shared temp directory is a name another user on the
/// same host can create first. When neither a state home nor a home directory is
/// available the caller gets `None`, and the REPL runs without history instead of
/// writing into a directory somebody else owns.
pub fn history_dir() -> Option<PathBuf> {
    history_dir_from(
        std::env::var_os("REDBCLI_HISTORY_DIR"),
        std::env::var_os("XDG_STATE_HOME"),
        std::env::var_os("HOME"),
    )
}

/// Pure resolver behind [`history_dir`], so the precedence is testable without
/// mutating the process environment (tests run in parallel).
pub fn history_dir_from(
    override_dir: Option<OsString>,
    xdg_state_home: Option<OsString>,
    home: Option<OsString>,
) -> Option<PathBuf> {
    if let Some(dir) = override_dir.filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    if let Some(xdg) = xdg_state_home.filter(|value| !value.is_empty()) {
        let path = PathBuf::from(&xdg);
        if path.is_absolute() {
            return Some(path.join("redbcli"));
        }
    }
    home.filter(|value| !value.is_empty()).map(|home| {
        PathBuf::from(home)
            .join(".local")
            .join("state")
            .join("redbcli")
    })
}

/// Create the history directory and file, returning `None` instead of panicking.
///
/// A REPL with no history is a degraded but usable CLI; a panic in `main` before
/// the first prompt means the tool cannot be used at all on a read-only or
/// foreign-owned `$HOME` or `XDG_STATE_HOME`.
///
/// The directory is created owner-only (`0700`). An existing one is refused when
/// it is a symlink, not a directory, or writable by group/other: history another
/// user can add to or replace is not history. The file is created with
/// `create_new` — never truncating one another session just wrote — at mode
/// `0600` from the first byte, so no descriptor can be held open during a window
/// when the file is still `0644`. An existing file is reused only when it is a
/// regular file: a symlink there would redirect the history writes.
pub fn prepare_history_file(dir: &Path) -> Option<PathBuf> {
    if let Err(err) = create_history_dir(dir) {
        eprintln!(
            "warning: cannot create history directory {}: {err}",
            dir.display()
        );
        return None;
    }
    if !directory_is_private(dir) {
        eprintln!(
            "warning: refusing history directory {}: not a private directory",
            dir.display()
        );
        return None;
    }
    let file = dir.join("history.txt");
    match create_history_file(&file) {
        Ok(()) => Some(file),
        // Another REPL created it between the check and our open: reuse it rather
        // than truncating what that session already saved.
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
            match reuse_history_file(&file) {
                Ok(()) => Some(file),
                Err(reason) => {
                    eprintln!(
                        "warning: refusing history file {}: {reason}",
                        file.display()
                    );
                    None
                }
            }
        }
        Err(err) => {
            eprintln!(
                "warning: cannot create history file {}: {err}",
                file.display()
            );
            None
        }
    }
}

#[cfg(unix)]
fn create_history_dir(dir: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
}

#[cfg(not(unix))]
fn create_history_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)
}

/// False when the directory could be controlled by somebody else.
///
/// Mode bits are a Unix notion; elsewhere only the "is a directory" part is
/// checked, which is what `std` can express portably.
fn directory_is_private(dir: &Path) -> bool {
    let Ok(meta) = std::fs::symlink_metadata(dir) else {
        return false;
    };
    if !meta.file_type().is_dir() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o022 != 0 {
            return false;
        }
    }
    true
}

fn create_history_file(file: &Path) -> std::io::Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(file).map(|_| ())
}

/// Validate an existing history file, tightening permissions an older version left.
///
/// The error carries the reason a caller must report: "cannot tighten" is not the
/// same as "not a regular file", and silently keeping a file other users can read
/// would leave the history exposed for the rest of the session.
fn reuse_history_file(file: &Path) -> Result<(), String> {
    let Ok(meta) = std::fs::symlink_metadata(file) else {
        return Err("cannot inspect it".to_string());
    };
    if !meta.file_type().is_file() {
        return Err("not a regular file".to_string());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o077 != 0 {
            std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o600))
                .map_err(|err| format!("readable by group/other and cannot be tightened: {err}"))?;
        }
    }
    Ok(())
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
        );
        assert_eq!(dir, Some(PathBuf::from("/srv/state/cli")));
    }

    #[test]
    fn history_dir_uses_an_absolute_xdg_state_home() {
        // Build the fixture natively: "/xdg/state" has no drive on Windows and is
        // therefore not absolute there, so the resolver would skip it.
        let xdg = std::env::temp_dir().join("xdg-state");
        let dir = history_dir_from(
            None,
            Some(xdg.clone().into_os_string()),
            Some(OsString::from("/home/app")),
        );
        assert_eq!(dir, Some(xdg.join("redbcli")));
    }

    #[test]
    fn history_dir_ignores_a_relative_xdg_state_home() {
        // The XDG spec says a relative value is invalid and must be ignored.
        let dir = history_dir_from(
            None,
            Some(OsString::from("relative/state")),
            Some(OsString::from("/home/app")),
        );
        assert_eq!(dir, Some(PathBuf::from("/home/app/.local/state/redbcli")));
    }

    #[test]
    fn history_dir_ignores_empty_values() {
        let dir = history_dir_from(
            Some(OsString::new()),
            Some(OsString::new()),
            Some(OsString::new()),
        );
        assert_eq!(dir, None);
    }

    #[test]
    fn history_dir_falls_back_to_home_and_then_stops() {
        let with_home = history_dir_from(None, None, Some(OsString::from("/home/app")));
        assert_eq!(
            with_home,
            Some(PathBuf::from("/home/app/.local/state/redbcli"))
        );

        // No shared temp directory: a predictable path there can be pre-created by
        // another user, so history is disabled instead.
        assert_eq!(history_dir_from(None, None, None), None);
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

    #[cfg(unix)]
    #[test]
    fn prepare_history_file_creates_the_file_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("state");
        let file = prepare_history_file(&dir).expect("history file should be created");

        let dir_mode = std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
        let file_mode = std::fs::metadata(&file).unwrap().permissions().mode() & 0o777;
        // 0700 & !umask, and 0600 regardless of umask: no group/other access.
        assert_eq!(dir_mode & 0o077, 0, "directory mode {dir_mode:o}");
        assert_eq!(file_mode & 0o077, 0, "file mode {file_mode:o}");
    }

    #[cfg(unix)]
    #[test]
    fn prepare_history_file_refuses_a_world_writable_directory() {
        use std::os::unix::fs::PermissionsExt;

        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("shared");
        std::fs::create_dir(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o777)).unwrap();

        // Another user could add or replace the history in a 0777 directory.
        assert!(prepare_history_file(&dir).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn prepare_history_file_refuses_a_symlinked_history_file() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("state");
        std::fs::create_dir(&dir).unwrap();
        let target = root.path().join("elsewhere.txt");
        std::fs::write(&target, "").unwrap();
        std::os::unix::fs::symlink(&target, dir.join("history.txt")).unwrap();

        assert!(prepare_history_file(&dir).is_none());
        // The target is untouched: the refusal must not write through the link.
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "");
    }

    #[cfg(unix)]
    #[test]
    fn prepare_history_file_tightens_a_legacy_file() {
        use std::os::unix::fs::PermissionsExt;

        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("state");
        std::fs::create_dir(&dir).unwrap();
        // The directory itself is ours and private; only the file is legacy.
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        let file = dir.join("history.txt");
        std::fs::write(&file, "set db.redb\n").unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();

        assert_eq!(prepare_history_file(&dir), Some(file.clone()));
        let mode = std::fs::metadata(&file).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode & 0o077, 0, "file mode {mode:o}");
        // Reused, not truncated.
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "set db.redb\n");
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
