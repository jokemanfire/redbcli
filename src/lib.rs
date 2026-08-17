use std::collections::HashMap;
use std::io::Write;
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
    fn json_preserves_tuple_and_hex_cells() {
        let rows = vec![
            ("(1, 2)".to_string(), "0x01abff".to_string()),
            ("(3, 4)".to_string(), "100".to_string()),
        ];
        let parsed = json_to_rows(&rows_to_json(&rows).unwrap()).unwrap();
        assert_eq!(parsed, rows);
    }
}
