//! Regression tests for issue #1: tables whose value is a 4-element tuple.
//!
//! These used to fail with
//! `DB corrupted: table 'ranges' has unsupported key/value types Table<u32, (u128,u128,u128,u128)>`.

use redb::{Database, TableDefinition};
use redbcli::redbcontrol::CommonDbManager;

const RANGES: TableDefinition<u32, (u128, u128, u128, u128)> = TableDefinition::new("ranges");
const STR_TO_QUAD: TableDefinition<&str, (u128, u128, u128, u128)> =
    TableDefinition::new("str_to_quad");
const QUAD_TO_U64: TableDefinition<(u64, u64, u64, u64), u64> = TableDefinition::new("quad_to_u64");

fn make_db() -> (tempfile::TempDir, CommonDbManager) {
    let dir = tempfile::tempdir().expect("create temp dir failed");
    let path = dir.path().join("issue1.rdb");
    let db = Database::create(&path).expect("create db failed");

    let txn = db.begin_write().expect("begin write failed");
    {
        let mut t = txn.open_table(RANGES).unwrap();
        t.insert(7u32, (10u128, 20u128, 30u128, 40u128)).unwrap();
    }
    {
        let mut t = txn.open_table(STR_TO_QUAD).unwrap();
        t.insert("acme", (1u128, 2u128, 3u128, 4u128)).unwrap();
    }
    {
        // 4-element tuple keys are intentionally not supported yet; this table
        // only proves the reader reports a clear, non-"corrupted" error.
        let mut t = txn.open_table(QUAD_TO_U64).unwrap();
        t.insert((1u64, 2u64, 3u64, 4u64), 99u64).unwrap();
    }
    txn.commit().unwrap();
    drop(db);

    let mut dbm = CommonDbManager::default();
    dbm.setdbpath(path.to_string_lossy().to_string())
        .expect("set db path failed");
    (dir, dbm)
}

#[test]
fn reads_four_element_tuple_values() {
    let (_dir, dbm) = make_db();
    let (desc, rows) = dbm.get_all_dyn("ranges").expect("read ranges failed");
    assert_eq!(desc.key, "u32");
    assert_eq!(desc.value, "(u128,u128,u128,u128)");
    assert_eq!(
        rows,
        vec![("7".to_string(), "(10, 20, 30, 40)".to_string())]
    );
}

#[test]
fn reads_four_element_tuple_values_with_str_keys() {
    let (_dir, dbm) = make_db();
    let rows = dbm.get_all_dyn("str_to_quad").expect("read failed").1;
    assert_eq!(rows, vec![("acme".to_string(), "(1, 2, 3, 4)".to_string())]);
}

#[test]
fn edits_four_element_tuple_values() {
    let (_dir, dbm) = make_db();
    let mut rows = dbm.get_all_dyn("ranges").expect("read failed").1;
    rows[0].1 = "(11, 21, 31, 41)".to_string();
    rows.push(("8".to_string(), "(0, 0, 0, 0)".to_string()));
    dbm.update_all_dyn("ranges", rows).expect("write failed");

    assert_eq!(
        dbm.get_all_dyn("ranges").expect("re-read failed").1,
        vec![
            ("7".to_string(), "(11, 21, 31, 41)".to_string()),
            ("8".to_string(), "(0, 0, 0, 0)".to_string()),
        ]
    );
}

#[test]
fn rejects_invalid_four_element_tuple_text() {
    let (_dir, dbm) = make_db();
    let rows = vec![("7".to_string(), "(1, 2, 3)".to_string())];
    let err = dbm.update_all_dyn("ranges", rows).unwrap_err().to_string();
    assert!(err.contains("4 tuple elements"), "unexpected error: {err}");
    // Original value is untouched.
    assert_eq!(
        dbm.get_all_dyn("ranges").unwrap().1,
        vec![("7".to_string(), "(10, 20, 30, 40)".to_string())]
    );
}

#[test]
fn unsupported_types_report_a_clear_error() {
    let (_dir, dbm) = make_db();

    // 4-element tuple keys are still outside the registry.
    let err = dbm.get_all_dyn("quad_to_u64").unwrap_err().to_string();
    assert!(err.contains("cannot decode yet"), "unexpected error: {err}");
    assert!(
        err.contains("(u64,u64,u64,u64)"),
        "error should name the actual type: {err}"
    );

    // 5- and 6-element tuple values are supported now; issue_4_tuple_arity.rs
    // covers them and the arity that is still outside the registry.
}
