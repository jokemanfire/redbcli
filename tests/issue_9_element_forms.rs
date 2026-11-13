//! Regression tests for issue #9: a value's text form must not depend on
//! whether it is the whole cell or an element of a tuple, and every printed
//! cell must parse back verbatim.

use redb::{Database, TableDefinition};
use redbcli::redbcontrol::CommonDbManager;

const SCALAR_BYTES: TableDefinition<u32, &'static [u8]> = TableDefinition::new("scalar_bytes");
#[allow(clippy::type_complexity)]
const TUPLE_BYTES: TableDefinition<
    u32,
    (&'static [u8], &'static [u8], &'static [u8], &'static [u8]),
> = TableDefinition::new("tuple_bytes");
const SCALAR_STR: TableDefinition<u32, &'static str> = TableDefinition::new("scalar_str");
const STR_PAIR: TableDefinition<(&'static str, &'static str), u64> =
    TableDefinition::new("str_pair");
#[allow(clippy::type_complexity)]
const TUPLE_STR: TableDefinition<&str, (String, String, String, String, String)> =
    TableDefinition::new("tuple_str");

fn make_db() -> (tempfile::TempDir, CommonDbManager) {
    let dir = tempfile::tempdir().expect("create temp dir failed");
    let path = dir.path().join("issue9.rdb");
    let db = Database::create(&path).expect("create db failed");

    let txn = db.begin_write().expect("begin write failed");
    {
        let mut t = txn.open_table(SCALAR_BYTES).unwrap();
        t.insert(1u32, b"aa".as_slice()).unwrap();
    }
    {
        let mut t = txn.open_table(TUPLE_BYTES).unwrap();
        t.insert(
            1u32,
            (
                b"aa".as_slice(),
                b"bb".as_slice(),
                b"cc".as_slice(),
                b"dd".as_slice(),
            ),
        )
        .unwrap();
    }
    {
        let mut t = txn.open_table(SCALAR_STR).unwrap();
        t.insert(1u32, "aa").unwrap();
    }
    {
        // 2-element tuples are only registered as *keys*.
        let mut t = txn.open_table(STR_PAIR).unwrap();
        t.insert(("left", "right"), 7u64).unwrap();
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
    (dir, dbm)
}

fn rows(dbm: &CommonDbManager, table: &str) -> Vec<(String, String)> {
    dbm.get_all_dyn(table).expect("read failed").1
}

fn update(dbm: &CommonDbManager, table: &str, key: &str, value: &str) {
    dbm.update_all_dyn(table, vec![(key.to_string(), value.to_string())])
        .expect("write failed");
}

#[test]
fn a_byte_element_is_shown_as_hex_inside_a_tuple() {
    let (_dir, dbm) = make_db();
    assert_eq!(
        rows(&dbm, "tuple_bytes"),
        vec![(
            "1".to_string(),
            "(0x6161, 0x6262, 0x6363, 0x6464)".to_string()
        )]
    );
}

#[test]
fn a_scalar_byte_value_also_accepts_the_decimal_list_form() {
    let (_dir, dbm) = make_db();
    update(&dbm, "scalar_bytes", "1", "[97, 97]");
    assert_eq!(
        rows(&dbm, "scalar_bytes"),
        vec![("1".to_string(), "0x6161".to_string())]
    );
}

#[test]
fn a_tuple_byte_element_also_accepts_the_hex_form() {
    let (_dir, dbm) = make_db();
    update(&dbm, "tuple_bytes", "1", "(0x6161, 0x6262, 0x6363, 0x6464)");
    assert_eq!(
        rows(&dbm, "tuple_bytes"),
        vec![(
            "1".to_string(),
            "(0x6161, 0x6262, 0x6363, 0x6464)".to_string()
        )]
    );
    // And the decimal list form still works, so old cells keep parsing.
    update(
        &dbm,
        "tuple_bytes",
        "1",
        "([97, 97], [98, 98], [99, 99], [100, 100])",
    );
    assert_eq!(
        rows(&dbm, "tuple_bytes"),
        vec![(
            "1".to_string(),
            "(0x6161, 0x6262, 0x6363, 0x6464)".to_string()
        )]
    );
}

#[test]
fn a_quoted_scalar_string_does_not_store_its_quotes() {
    let (_dir, dbm) = make_db();
    update(&dbm, "scalar_str", "1", "\"hello\"");
    assert_eq!(
        rows(&dbm, "scalar_str"),
        vec![("1".to_string(), "hello".to_string())]
    );

    // Escapes inside the quotes are honoured, as they are in a tuple cell.
    update(&dbm, "scalar_str", "1", "\"a\\\"b\"");
    assert_eq!(
        rows(&dbm, "scalar_str"),
        vec![("1".to_string(), "a\"b".to_string())]
    );

    // Quotes that do not delimit the cell stay data: no silent rewrite.
    update(&dbm, "scalar_str", "1", "a\"b");
    assert_eq!(
        rows(&dbm, "scalar_str"),
        vec![("1".to_string(), "a\"b".to_string())]
    );
}

#[test]
fn a_tuple_string_element_also_accepts_the_bare_form() {
    let (_dir, dbm) = make_db();
    update(&dbm, "tuple_str", "k", "(a, b, c, d, e)");
    assert_eq!(
        rows(&dbm, "tuple_str"),
        vec![(
            "k".to_string(),
            "(\"a\", \"b\", \"c\", \"d\", \"e\")".to_string()
        )]
    );
}

#[test]
fn a_borrowed_string_tuple_accepts_both_element_forms() {
    let (_dir, dbm) = make_db();
    let printed = vec![("(\"left\", \"right\")".to_string(), "7".to_string())];
    assert_eq!(rows(&dbm, "str_pair"), printed);

    // Bare, the form a scalar cell prints.
    dbm.update_all_dyn(
        "str_pair",
        vec![("(left, right)".to_string(), "7".to_string())],
    )
    .expect("bare key rejected");
    assert_eq!(rows(&dbm, "str_pair"), printed);

    // Quoted, the form a tuple key prints.
    dbm.update_all_dyn(
        "str_pair",
        vec![("(\"left\", \"right\")".to_string(), "8".to_string())],
    )
    .expect("quoted key rejected");
    assert_eq!(
        rows(&dbm, "str_pair"),
        vec![("(\"left\", \"right\")".to_string(), "8".to_string())]
    );
}

#[test]
fn every_printed_cell_is_accepted_verbatim() {
    let (_dir, dbm) = make_db();
    for table in [
        "scalar_bytes",
        "tuple_bytes",
        "scalar_str",
        "str_pair",
        "tuple_str",
    ] {
        let printed = rows(&dbm, table);
        dbm.update_all_dyn(table, printed.clone())
            .unwrap_or_else(|e| panic!("{table}: printed rows rejected: {e}"));
        assert_eq!(
            rows(&dbm, table),
            printed,
            "{table} lost data on a no-op edit"
        );
    }
}
