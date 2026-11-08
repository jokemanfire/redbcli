//! Regression tests for issue #3: a table whose key/value types are outside
//! the compiled-in registry is a *limitation of this build*, not database
//! corruption. The error must not be rendered through `redb::Error::Corrupted`,
//! because that prefix tells users their database file is damaged.

use redb::{Database, ReadableDatabase, TableDefinition};
use redbcli::redbcontrol::CommonDbManager;

const OPT_VALS: TableDefinition<u64, Option<u64>> = TableDefinition::new("opt_vals");

fn make_db() -> (tempfile::TempDir, CommonDbManager) {
    let dir = tempfile::tempdir().expect("create temp dir failed");
    let path = dir.path().join("issue3.rdb");
    let db = Database::create(&path).expect("create db failed");
    let txn = db.begin_write().expect("begin write failed");
    {
        let mut t = txn.open_table(OPT_VALS).unwrap();
        t.insert(1u64, Some(10u64)).unwrap();
        t.insert(2u64, None).unwrap();
    }
    txn.commit().unwrap();
    drop(db);

    let mut dbm = CommonDbManager::default();
    dbm.setdbpath(path.to_string_lossy().to_string())
        .expect("set db path failed");
    (dir, dbm)
}

#[test]
fn unsupported_type_is_not_reported_as_corruption() {
    let (_dir, dbm) = make_db();
    let err = dbm.get_all_dyn("opt_vals").unwrap_err();
    let msg = err.to_string();
    assert!(
        !msg.to_lowercase().contains("corrupt"),
        "unsupported type must not claim the DB is corrupted: {msg}"
    );
    assert!(
        msg.contains("Option<u64>"),
        "error should name the type: {msg}"
    );
    assert!(
        msg.contains("unsupported"),
        "error should say the type is unsupported: {msg}"
    );
}

#[test]
fn unsupported_type_edit_is_not_reported_as_corruption() {
    let (dir, dbm) = make_db();
    let err = dbm
        .update_all_dyn("opt_vals", vec![("1".to_string(), "11".to_string())])
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        !msg.to_lowercase().contains("corrupt"),
        "unsupported type must not claim the DB is corrupted: {msg}"
    );
    assert!(
        msg.contains("Option<u64>"),
        "error should name the type: {msg}"
    );

    // the table must be untouched
    let db = Database::open(dir.path().join("issue3.rdb")).unwrap();
    let txn = db.begin_read().unwrap();
    let table = txn.open_table(OPT_VALS).unwrap();
    assert_eq!(table.get(1u64).unwrap().unwrap().value(), Some(10u64));
}

#[test]
fn real_corruption_keeps_the_redb_rendering() {
    // Everything that is not the unsupported-type branch must keep redb's own
    // rendering, so a genuinely damaged file is still reported as corruption.
    let err: redbcli::CliError = redb::Error::Corrupted("bad page checksum".to_string()).into();
    let msg = err.to_string();
    assert!(msg.contains("corrupt"), "got: {msg}");
    assert!(msg.contains("bad page checksum"), "got: {msg}");
}
