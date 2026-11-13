//! Regression tests for the review of PR #10: a printed cell must parse back to
//! the value it was printed from, for every registered type, including values
//! whose own text begins and ends with a quote character.

use redb::{Database, ReadableDatabase, ReadableTableMetadata, TableDefinition};
use redbcli::redbcontrol::CommonDbManager;

const SCALAR_STR: TableDefinition<u32, &'static str> = TableDefinition::new("scalar_str");
const STR_KEY: TableDefinition<&'static str, u64> = TableDefinition::new("str_key");
const SCALAR_BYTES: TableDefinition<u32, &'static [u8]> = TableDefinition::new("scalar_bytes");
#[allow(clippy::type_complexity)]
const TUPLE_STR: TableDefinition<&str, (String, String, String, String, String)> =
    TableDefinition::new("tuple_str");

fn make_db() -> (tempfile::TempDir, CommonDbManager, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("create temp dir failed");
    let path = dir.path().join("roundtrip.rdb");
    let db = Database::create(&path).expect("create db failed");

    let txn = db.begin_write().expect("begin write failed");
    {
        let mut t = txn.open_table(SCALAR_STR).unwrap();
        // The stored value is `"hello"`, quotes included.
        t.insert(1u32, "\"hello\"").unwrap();
    }
    {
        let mut t = txn.open_table(STR_KEY).unwrap();
        // The key is `"k"`, quotes included.
        t.insert("\"k\"", 7u64).unwrap();
    }
    {
        let mut t = txn.open_table(SCALAR_BYTES).unwrap();
        t.insert(1u32, b"aa".as_slice()).unwrap();
    }
    {
        let mut t = txn.open_table(TUPLE_STR).unwrap();
        t.insert(
            "k",
            (
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "d".to_string(),
                "e".to_string(),
            ),
        )
        .unwrap();
    }
    txn.commit().unwrap();
    drop(db);

    let mut dbm = CommonDbManager::default();
    dbm.setdbpath(path.to_string_lossy().to_string())
        .expect("set db path failed");
    (dir, dbm, path)
}

fn rows(dbm: &CommonDbManager, table: &str) -> Vec<(String, String)> {
    dbm.get_all_dyn(table).expect("read failed").1
}

#[test]
fn a_no_op_save_does_not_strip_quotes_from_a_stored_value() {
    let (_dir, dbm, path) = make_db();
    let printed = rows(&dbm, "scalar_str");

    dbm.update_all_dyn("scalar_str", printed.clone())
        .expect("write failed");

    let db = Database::open(&path).unwrap();
    let txn = db.begin_read().unwrap();
    let table = txn.open_table(SCALAR_STR).unwrap();
    let stored = table.get(1u32).unwrap().unwrap();
    assert_eq!(stored.value(), "\"hello\"");
}

#[test]
fn a_no_op_save_does_not_leave_the_old_quoted_key_behind() {
    let (_dir, dbm, path) = make_db();
    let printed = rows(&dbm, "str_key");

    dbm.update_all_dyn("str_key", printed)
        .expect("write failed");

    let db = Database::open(&path).unwrap();
    let txn = db.begin_read().unwrap();
    let table = txn.open_table(STR_KEY).unwrap();
    assert_eq!(table.len().unwrap(), 1);
    let value = table.get("\"k\"").unwrap().expect("key lost its quotes");
    assert_eq!(value.value(), 7u64);
}

#[test]
fn a_bare_tuple_string_may_contain_an_apostrophe() {
    let (_dir, dbm, _path) = make_db();

    dbm.update_all_dyn(
        "tuple_str",
        vec![("k".to_string(), "(don't, b, c, d, e)".to_string())],
    )
    .expect("write failed");

    let after = rows(&dbm, "tuple_str");
    assert_eq!(after[0].1, "(\"don't\", \"b\", \"c\", \"d\", \"e\")");
}

#[test]
fn a_bare_tuple_string_may_contain_a_double_quote() {
    let (_dir, dbm, _path) = make_db();

    dbm.update_all_dyn(
        "tuple_str",
        vec![("k".to_string(), "(a\"b, b, c, d, e)".to_string())],
    )
    .expect("write failed");

    let after = rows(&dbm, "tuple_str");
    assert_eq!(after[0].1, "(\"a\\\"b\", \"b\", \"c\", \"d\", \"e\")");
}

#[test]
fn a_non_ascii_hex_cell_is_an_error_not_a_panic() {
    let (_dir, dbm, path) = make_db();

    let err = dbm
        .update_all_dyn("scalar_bytes", vec![("1".to_string(), "0x0€".to_string())])
        .expect_err("non-ASCII hex must be rejected");
    assert!(
        err.to_string().contains("not all hex digits"),
        "unexpected error: {err}"
    );

    let db = Database::open(&path).unwrap();
    let txn = db.begin_read().unwrap();
    let table = txn.open_table(SCALAR_BYTES).unwrap();
    assert_eq!(table.get(1u32).unwrap().unwrap().value(), b"aa");
}
