use redb::{Database, MultimapTableDefinition, TableDefinition};
use redbcli::redbcontrol::{CommonDbManager, DealData, DealTable};

const STR_TABLE: TableDefinition<&str, &str> = TableDefinition::new("strings");
const FOUND_KEYS: TableDefinition<(u128, u128), u64> = TableDefinition::new("found_keys");
const BY_ID: TableDefinition<u64, &str> = TableDefinition::new("by_id");
const BLOBS: TableDefinition<u32, &[u8]> = TableDefinition::new("blobs");
const MM_TABLE: MultimapTableDefinition<&str, u64> = MultimapTableDefinition::new("tags");
const OPTION_VALS: TableDefinition<u64, Option<u64>> = TableDefinition::new("opt_vals");

fn make_db() -> (tempfile::TempDir, CommonDbManager) {
    let dir = tempfile::tempdir().expect("create temp dir failed");
    let path = dir.path().join("test.redb");
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
        let mut t = txn.open_multimap_table(MM_TABLE).unwrap();
        t.insert("k", 1u64).unwrap();
    }
    {
        let mut t = txn.open_table(OPTION_VALS).unwrap();
        t.insert(5u64, Some(9u64)).unwrap();
    }
    txn.commit().unwrap();
    drop(db);

    let mut dbm = CommonDbManager::default();
    dbm.setdbpath(path.to_string_lossy().to_string())
        .expect("set db path failed");
    (dir, dbm)
}

#[test]
fn list_table_finds_all_normal_tables() {
    let (_dir, dbm) = make_db();
    let mut tables = dbm.list_table().unwrap();
    tables.sort();
    assert_eq!(
        tables,
        vec!["blobs", "by_id", "found_keys", "opt_vals", "strings"]
    );
}

#[test]
fn use_table_accepts_non_str_types() {
    let (_dir, mut dbm) = make_db();
    dbm.settablename("found_keys".to_string()).unwrap();
    assert_eq!(dbm.tablename, "found_keys");
}

#[test]
fn use_table_rejects_missing_table() {
    let (_dir, mut dbm) = make_db();
    let err = dbm.settablename("nope".to_string()).unwrap_err();
    assert!(err.to_string().contains("does not exist"));
}

#[test]
fn use_table_rejects_multimap_table() {
    let (_dir, mut dbm) = make_db();
    let err = dbm.settablename("tags".to_string()).unwrap_err();
    assert!(err.to_string().contains("multimap"));
}

#[test]
fn table_type_reports_persisted_types() {
    let (_dir, dbm) = make_db();
    let desc = dbm.table_type("found_keys").unwrap();
    assert_eq!(desc.key, "(u128,u128)");
    assert_eq!(desc.value, "u64");
    let desc = dbm.table_type("strings").unwrap();
    assert_eq!(desc.key, "&str");
    assert_eq!(desc.value, "&str");
}

#[test]
fn list_table_types_reports_all_types() {
    let (_dir, dbm) = make_db();
    let types = dbm.list_table_types().unwrap();
    let as_map: std::collections::HashMap<_, _> = types.into_iter().collect();
    assert_eq!(as_map["found_keys"].key, "(u128,u128)");
    assert_eq!(as_map["found_keys"].value, "u64");
    assert_eq!(as_map["by_id"].key, "u64");
    assert_eq!(as_map["by_id"].value, "&str");
    assert_eq!(as_map["blobs"].value, "&[u8]");
}

#[test]
fn info_table_reads_tuple_key_table() {
    let (_dir, dbm) = make_db();
    let (desc, rows) = dbm.get_all_dyn("found_keys").unwrap();
    assert_eq!(desc.key, "(u128,u128)");
    assert_eq!(desc.value, "u64");
    assert_eq!(
        rows,
        vec![
            ("(1, 2)".to_string(), "100".to_string()),
            (format!("({}, 7)", u128::MAX), "42".to_string()),
        ]
    );
}

#[test]
fn info_table_reads_scalar_key_table() {
    let (_dir, dbm) = make_db();
    let (_desc, rows) = dbm.get_all_dyn("by_id").unwrap();
    assert_eq!(
        rows,
        vec![
            ("1".to_string(), "one".to_string()),
            ("2".to_string(), "two".to_string()),
        ]
    );
}

#[test]
fn info_table_decodes_byte_values_as_hex() {
    let (_dir, dbm) = make_db();
    let (_desc, rows) = dbm.get_all_dyn("blobs").unwrap();
    assert_eq!(rows, vec![("7".to_string(), "0x01abff".to_string())]);
}

#[test]
fn info_table_still_reads_str_tables() {
    let (_dir, dbm) = make_db();
    let (desc, rows) = dbm.get_all_dyn("strings").unwrap();
    assert_eq!(desc.key, "&str");
    assert_eq!(rows.len(), 2);
    assert!(rows.contains(&("hello".to_string(), "world".to_string())));
    assert!(rows.contains(&("foo".to_string(), "bar".to_string())));
}

#[test]
fn info_table_reports_unsupported_types() {
    let (_dir, dbm) = make_db();
    let err = dbm.get_all_dyn("opt_vals").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("unsupported"), "unexpected error: {msg}");
    assert!(msg.contains("Option<u64>"), "unexpected error: {msg}");
}

#[test]
fn get_by_key_roundtrip_on_str_table() {
    let (_dir, mut dbm) = make_db();
    dbm.settablename("strings".to_string()).unwrap();
    assert_eq!(dbm.get_by_key("hello".to_string()).unwrap(), "world");
    let err = dbm.get_by_key("missing".to_string()).unwrap_err();
    assert!(err.to_string().contains("Key not found"));
}

#[test]
fn get_by_key_on_non_str_table_gives_friendly_error() {
    let (_dir, mut dbm) = make_db();
    dbm.settablename("found_keys".to_string()).unwrap();
    let err = dbm.get_by_key("1".to_string()).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("Table<(u128,u128), u64>"),
        "unexpected error: {msg}"
    );
    assert!(msg.contains("info table"), "unexpected error: {msg}");
}

#[test]
fn insert_update_remove_roundtrip_on_str_table() {
    let (_dir, mut dbm) = make_db();
    dbm.settablename("strings".to_string()).unwrap();

    dbm.insert_by_key("k".to_string(), "v1".to_string())
        .unwrap();
    assert_eq!(dbm.get_by_key("k".to_string()).unwrap(), "v1");

    dbm.update_by_key("k".to_string(), "v2".to_string())
        .unwrap();
    assert_eq!(dbm.get_by_key("k".to_string()).unwrap(), "v2");

    dbm.remove_by_key("k".to_string()).unwrap();
    assert!(dbm.get_by_key("k".to_string()).is_err());
}

#[test]
fn write_on_non_str_table_gives_friendly_error() {
    let (_dir, mut dbm) = make_db();
    dbm.settablename("by_id".to_string()).unwrap();
    let err = dbm
        .insert_by_key("1".to_string(), "x".to_string())
        .unwrap_err();
    assert!(
        err.to_string().contains("Table<u64, &str>"),
        "unexpected error: {err}"
    );
}

#[test]
fn edit_guard_rejects_non_str_table() {
    let (_dir, mut dbm) = make_db();
    dbm.settablename("by_id".to_string()).unwrap();
    let desc = dbm.table_type(&dbm.tablename).unwrap();
    assert!(desc.key != "&str" || desc.value != "&str");
}

#[test]
fn create_and_delete_table_roundtrip() {
    let (_dir, dbm) = make_db();
    dbm.create_table("temp".to_string()).unwrap();
    assert!(dbm.list_table().unwrap().contains(&"temp".to_string()));
    dbm.delete_table("temp".to_string()).unwrap();
    assert!(!dbm.list_table().unwrap().contains(&"temp".to_string()));
}

#[test]
fn regression_issue1_tuple_u128_table_is_browsable() {
    let (_dir, dbm) = make_db();
    let (desc, rows) = dbm.get_all_dyn("found_keys").unwrap();
    assert_eq!(desc.key, "(u128,u128)");
    assert_eq!(desc.value, "u64");
    assert_eq!(rows.len(), 2);
}
