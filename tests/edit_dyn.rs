use redb::{Database, ReadableDatabase, TableDefinition};
use redbcli::redbcontrol::CommonDbManager;

const STR_TABLE: TableDefinition<&str, &str> = TableDefinition::new("strings");
const FOUND_KEYS: TableDefinition<(u128, u128), u64> = TableDefinition::new("found_keys");
const BY_ID: TableDefinition<u64, &str> = TableDefinition::new("by_id");
const BLOBS: TableDefinition<u32, &[u8]> = TableDefinition::new("blobs");
const STR_PAIRS: TableDefinition<(&str, &str), &str> = TableDefinition::new("str_pairs");
const FLOATS: TableDefinition<u64, f64> = TableDefinition::new("floats");
const OPT_VALS: TableDefinition<u64, Option<u64>> = TableDefinition::new("opt_vals");

fn make_db() -> (tempfile::TempDir, CommonDbManager) {
    let dir = tempfile::tempdir().expect("create temp dir failed");
    let path = dir.path().join("edit.rdb");
    let db = Database::create(&path).expect("create db failed");

    let txn = db.begin_write().expect("begin write failed");
    {
        let mut t = txn.open_table(STR_TABLE).unwrap();
        t.insert("hello", "world").unwrap();
        t.insert("foo", "bar").unwrap();
    }
    {
        let mut t = txn.open_table(FOUND_KEYS).unwrap();
        t.insert((1u128, 2u128), 100u64).unwrap();
        t.insert((555u128, 666u128), 789u64).unwrap();
        t.insert((u128::MAX, 7u128), 42u64).unwrap();
    }
    {
        let mut t = txn.open_table(BY_ID).unwrap();
        t.insert(1u64, "one").unwrap();
        t.insert(2u64, "two").unwrap();
    }
    {
        let mut t = txn.open_table(BLOBS).unwrap();
        t.insert(7u32, &[0x01u8, 0xab, 0xff][..]).unwrap();
    }
    {
        let mut t = txn.open_table(STR_PAIRS).unwrap();
        t.insert(("alpha", "beta"), "ab").unwrap();
    }
    {
        let mut t = txn.open_table(FLOATS).unwrap();
        t.insert(1u64, 3.5f64).unwrap();
    }
    {
        let mut t = txn.open_table(OPT_VALS).unwrap();
        t.insert(5u64, Some(9u64)).unwrap();
    }
    txn.commit().unwrap();
    drop(db);

    let mut dbm = CommonDbManager::default();
    dbm.setdbpath(path.to_string_lossy().to_string())
        .expect("set db path failed");
    (dir, dbm)
}

fn rows_of(dbm: &CommonDbManager, table: &str) -> Vec<(String, String)> {
    dbm.get_all_dyn(table).unwrap().1
}

#[test]
fn edit_tuple_table_roundtrip() {
    let (_dir, dbm) = make_db();
    let mut rows = rows_of(&dbm, "found_keys");
    // modify one value, drop one row, add one new row
    assert_eq!(rows[0], ("(1, 2)".to_string(), "100".to_string()));
    rows[0].1 = "999".to_string();
    rows.retain(|(k, _)| k != "(555, 666)");
    rows.push(("(9, 9)".to_string(), "1".to_string()));

    dbm.update_all_dyn("found_keys", rows).unwrap();

    assert_eq!(
        rows_of(&dbm, "found_keys"),
        vec![
            ("(1, 2)".to_string(), "999".to_string()),
            ("(9, 9)".to_string(), "1".to_string()),
            (format!("({}, 7)", u128::MAX), "42".to_string()),
        ]
    );
}

#[test]
fn edit_scalar_key_table() {
    let (_dir, dbm) = make_db();
    dbm.update_all_dyn(
        "by_id",
        vec![
            ("1".to_string(), "ONE".to_string()),
            ("3".to_string(), "three".to_string()),
        ],
    )
    .unwrap();
    assert_eq!(
        rows_of(&dbm, "by_id"),
        vec![
            ("1".to_string(), "ONE".to_string()),
            ("3".to_string(), "three".to_string()),
        ]
    );
}

#[test]
fn edit_blob_hex_values() {
    let (_dir, dbm) = make_db();
    dbm.update_all_dyn(
        "blobs",
        vec![
            ("7".to_string(), "0xdeadbeef".to_string()),
            ("8".to_string(), "0x".to_string()),
        ],
    )
    .unwrap();
    assert_eq!(
        rows_of(&dbm, "blobs"),
        vec![
            ("7".to_string(), "0xdeadbeef".to_string()),
            ("8".to_string(), "0x".to_string()),
        ]
    );
}

#[test]
fn edit_str_table_regression() {
    let (_dir, dbm) = make_db();
    dbm.update_all_dyn(
        "strings",
        vec![("hello".to_string(), "new world".to_string())],
    )
    .unwrap();
    assert_eq!(
        rows_of(&dbm, "strings"),
        vec![("hello".to_string(), "new world".to_string())]
    );
}

#[test]
fn edit_str_tuple_keys() {
    let (_dir, dbm) = make_db();
    assert_eq!(
        rows_of(&dbm, "str_pairs"),
        vec![("(\"alpha\", \"beta\")".to_string(), "ab".to_string())]
    );
    dbm.update_all_dyn(
        "str_pairs",
        vec![
            ("(\"alpha\", \"beta\")".to_string(), "ab".to_string()),
            ("(\"a, b\", \"c\")".to_string(), "comma key".to_string()),
        ],
    )
    .unwrap();
    assert_eq!(
        rows_of(&dbm, "str_pairs"),
        vec![
            ("(\"a, b\", \"c\")".to_string(), "comma key".to_string()),
            ("(\"alpha\", \"beta\")".to_string(), "ab".to_string()),
        ]
    );
}

#[test]
fn edit_float_values() {
    let (_dir, dbm) = make_db();
    dbm.update_all_dyn("floats", vec![("1".to_string(), "2.25".to_string())])
        .unwrap();
    assert_eq!(
        rows_of(&dbm, "floats"),
        vec![("1".to_string(), "2.25".to_string())]
    );
}

#[test]
fn edit_empty_rows_clears_table() {
    let (_dir, dbm) = make_db();
    dbm.update_all_dyn("found_keys", vec![]).unwrap();
    assert_eq!(rows_of(&dbm, "found_keys"), vec![]);
}

#[test]
fn edit_invalid_value_keeps_table_unchanged() {
    let (_dir, dbm) = make_db();
    let before = rows_of(&dbm, "found_keys");
    let err = dbm
        .update_all_dyn(
            "found_keys",
            vec![("(1, 2)".to_string(), "not a number".to_string())],
        )
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("invalid value"), "got: {msg}");
    assert_eq!(rows_of(&dbm, "found_keys"), before);
}

#[test]
fn edit_invalid_key_keeps_table_unchanged() {
    let (_dir, dbm) = make_db();
    let before = rows_of(&dbm, "found_keys");
    let err = dbm
        .update_all_dyn("found_keys", vec![("bogus".to_string(), "5".to_string())])
        .unwrap_err();
    assert!(err.to_string().contains("invalid key"), "got: {err}");
    assert_eq!(rows_of(&dbm, "found_keys"), before);
}

#[test]
fn edit_out_of_range_value_keeps_table_unchanged() {
    let (_dir, dbm) = make_db();
    let before = rows_of(&dbm, "found_keys");
    assert!(dbm
        .update_all_dyn(
            "found_keys",
            vec![("(1, 2)".to_string(), "99999999999999999999".to_string())]
        )
        .is_err());
    assert_eq!(rows_of(&dbm, "found_keys"), before);
}

#[test]
fn edit_unsupported_type_reports_error() {
    let (dir, dbm) = make_db();
    let err = dbm
        .update_all_dyn("opt_vals", vec![("5".to_string(), "9".to_string())])
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("cannot be edited"), "got: {msg}");
    assert!(msg.contains("Option<u64>"), "got: {msg}");
    // verify data is untouched via a direct typed read
    let db = Database::open(dir.path().join("edit.rdb")).unwrap();
    let txn = db.begin_read().unwrap();
    let table = txn.open_table(OPT_VALS).unwrap();
    assert_eq!(table.get(5u64).unwrap().unwrap().value(), Some(9u64));
}

#[test]
fn edit_flow_via_json_helpers_end_to_end() {
    // simulates what the `edit` command does: rows -> json -> (edit) -> json -> rows -> write
    let (_dir, dbm) = make_db();
    let (_desc, rows) = dbm.get_all_dyn("found_keys").unwrap();
    let json = redbcli::rows_to_json(&rows).unwrap();

    let edited = json.replace("\"100\"", "\"777\"");
    let new_rows = redbcli::json_to_rows(&edited).unwrap();
    dbm.update_all_dyn("found_keys", new_rows).unwrap();

    let result = rows_of(&dbm, "found_keys");
    assert!(result.contains(&("(1, 2)".to_string(), "777".to_string())));
    assert!(!result.contains(&("(1, 2)".to_string(), "100".to_string())));
}
