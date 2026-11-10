//! Regression tests for issue #4: value-side tuple arity 5 and 6.
//!
//! The registry used to stop at 4-element tuple values (and only for non-tuple
//! keys), so `Table<u32, (u128,u128,u128,u128,u128)>` was reported as
//! undecodable. These tests pin the widened, documented matrix:
//!
//! * base keys: scalars/`&str`/`String`/`&[u8]` with 2-6 element tuple values;
//! * tuple keys: 2-3 element homogeneous tuples, with 2-3 element tuple values;
//! * everything wider stays a clear "cannot decode yet" error.

use redb::{Database, TableDefinition};
use redbcli::redbcontrol::CommonDbManager;

const FIVE: TableDefinition<u32, (u128, u128, u128, u128, u128)> = TableDefinition::new("five");
const SIX_BY_STR: TableDefinition<&str, (u64, u64, u64, u64, u64, u64)> =
    TableDefinition::new("six_by_str");
// The shape is the thing under test, so it stays spelled out.
#[allow(clippy::type_complexity)]
const SEVEN: TableDefinition<u32, (u128, u128, u128, u128, u128, u128, u128)> =
    TableDefinition::new("seven");
#[allow(clippy::type_complexity)]
const TWO_KEY_FIVE_VALUE: TableDefinition<(u64, u64), (u32, u32, u32, u32, u32)> =
    TableDefinition::new("two_key_five_value");

fn make_db() -> (tempfile::TempDir, CommonDbManager) {
    let dir = tempfile::tempdir().expect("create temp dir failed");
    let path = dir.path().join("issue4.rdb");
    let db = Database::create(&path).expect("create db failed");

    let txn = db.begin_write().expect("begin write failed");
    {
        let mut t = txn.open_table(FIVE).unwrap();
        t.insert(7u32, (10u128, 20u128, 30u128, 40u128, 50u128))
            .unwrap();
    }
    {
        let mut t = txn.open_table(SIX_BY_STR).unwrap();
        t.insert("acme", (1u64, 2u64, 3u64, 4u64, 5u64, 6u64))
            .unwrap();
    }
    {
        let mut t = txn.open_table(SEVEN).unwrap();
        t.insert(1u32, (1u128, 2u128, 3u128, 4u128, 5u128, 6u128, 7u128))
            .unwrap();
    }
    {
        // Tuple keys keep the narrower value coverage: 5-element values are only
        // registered for base (non-tuple) keys.
        let mut t = txn.open_table(TWO_KEY_FIVE_VALUE).unwrap();
        t.insert((1u64, 2u64), (1u32, 2u32, 3u32, 4u32, 5u32))
            .unwrap();
    }
    txn.commit().unwrap();
    drop(db);

    let mut dbm = CommonDbManager::default();
    dbm.setdbpath(path.to_string_lossy().to_string())
        .expect("set db path failed");
    (dir, dbm)
}

#[test]
fn reads_five_element_tuple_values() {
    let (_dir, dbm) = make_db();
    let (desc, rows) = dbm.get_all_dyn("five").expect("read five failed");

    assert_eq!(desc.key, "u32");
    assert_eq!(desc.value, "(u128,u128,u128,u128,u128)");
    assert_eq!(
        rows,
        vec![("7".to_string(), "(10, 20, 30, 40, 50)".to_string())]
    );
}

#[test]
fn reads_six_element_tuple_values_with_str_keys() {
    let (_dir, dbm) = make_db();
    let rows = dbm.get_all_dyn("six_by_str").expect("read failed").1;

    assert_eq!(
        rows,
        vec![("acme".to_string(), "(1, 2, 3, 4, 5, 6)".to_string())]
    );
}

#[test]
fn edits_six_element_tuple_values() {
    let (_dir, dbm) = make_db();
    let mut rows = dbm.get_all_dyn("six_by_str").expect("read failed").1;
    rows[0].1 = "(11, 21, 31, 41, 51, 61)".to_string();
    rows.push(("globex".to_string(), "(0, 0, 0, 0, 0, 0)".to_string()));
    dbm.update_all_dyn("six_by_str", rows)
        .expect("write failed");

    assert_eq!(
        dbm.get_all_dyn("six_by_str").expect("re-read failed").1,
        vec![
            ("acme".to_string(), "(11, 21, 31, 41, 51, 61)".to_string()),
            ("globex".to_string(), "(0, 0, 0, 0, 0, 0)".to_string()),
        ]
    );
}

#[test]
fn rejects_invalid_six_element_text() {
    let (_dir, dbm) = make_db();
    let rows = vec![("acme".to_string(), "(1, 2, 3, 4, 5)".to_string())];
    let err = dbm
        .update_all_dyn("six_by_str", rows)
        .unwrap_err()
        .to_string();

    assert!(err.contains("6 tuple elements"), "unexpected error: {err}");
    assert_eq!(
        dbm.get_all_dyn("six_by_str").unwrap().1,
        vec![("acme".to_string(), "(1, 2, 3, 4, 5, 6)".to_string())]
    );
}

#[test]
fn arity_seven_and_tuple_keys_with_wide_values_stay_unsupported() {
    let (_dir, dbm) = make_db();

    let err = dbm.get_all_dyn("seven").unwrap_err().to_string();
    assert!(err.contains("cannot decode yet"), "unexpected error: {err}");
    assert!(
        err.contains("(u128,u128,u128,u128,u128,u128,u128)"),
        "error should name the actual type: {err}"
    );

    let err = dbm
        .get_all_dyn("two_key_five_value")
        .unwrap_err()
        .to_string();
    assert!(err.contains("cannot decode yet"), "unexpected error: {err}");
}
