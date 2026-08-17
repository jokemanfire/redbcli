use redb::{
    Key, ReadTransaction, ReadableTable, TableDefinition, TableError, Value, WriteTransaction,
};
use std::borrow::Borrow;
use std::fmt::Debug;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableTypeDesc {
    pub key: String,
    pub value: String,
}

pub fn type_name_of<T: Value + 'static>() -> String {
    T::type_name().name().to_string()
}

/// Probes the persisted key/value types of a table by attempting a typed open.
pub fn probe_table_type(txn: &ReadTransaction, name: &str) -> Result<TableTypeDesc, TableError> {
    let definition: TableDefinition<&str, &str> = TableDefinition::new(name);
    match txn.open_table(definition) {
        Ok(_) => Ok(TableTypeDesc {
            key: "&str".to_string(),
            value: "&str".to_string(),
        }),
        Err(TableError::TableTypeMismatch { key, value, .. }) => Ok(TableTypeDesc {
            key: key.name().to_string(),
            value: value.name().to_string(),
        }),
        Err(e) => Err(e),
    }
}

// ---------------------------------------------------------------------------
// Formatting (table cells -> display strings)
// ---------------------------------------------------------------------------

trait FmtCell {
    fn fmt_cell(&self) -> String;
}

macro_rules! impl_fmt_cell_debug {
    ($($t:ty),*) => {
        $(impl FmtCell for $t {
            fn fmt_cell(&self) -> String {
                format!("{self:?}")
            }
        })*
    };
}

impl_fmt_cell_debug!(
    u8,
    u16,
    u32,
    u64,
    u128,
    i8,
    i16,
    i32,
    i64,
    i128,
    f32,
    f64,
    bool,
    char,
    ()
);

impl FmtCell for &str {
    fn fmt_cell(&self) -> String {
        self.to_string()
    }
}

impl FmtCell for String {
    fn fmt_cell(&self) -> String {
        self.clone()
    }
}

impl FmtCell for &[u8] {
    fn fmt_cell(&self) -> String {
        bytes_to_hex(self)
    }
}

impl<A: Debug, B: Debug> FmtCell for (A, B) {
    fn fmt_cell(&self) -> String {
        format!("{self:?}")
    }
}

impl<A: Debug, B: Debug, C: Debug> FmtCell for (A, B, C) {
    fn fmt_cell(&self) -> String {
        format!("{self:?}")
    }
}

pub fn bytes_to_hex(data: &[u8]) -> String {
    let mut out = String::with_capacity(2 + data.len() * 2);
    out.push_str("0x");
    for byte in data {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

// ---------------------------------------------------------------------------
// Parsing (edited strings -> typed values)
// ---------------------------------------------------------------------------

/// Owned representation of a redb key/value type, used to parse an edited
/// cell back into a typed argument for `insert`/`remove`.
///
/// Two parse entry points exist because formatting differs by context:
/// - `parse_cell`: inverse of `FmtCell::fmt_cell` (standalone cell)
/// - `parse_elem`: inverse of `Debug` formatting inside tuples (quoted
///   strings, `[1, 2]` byte arrays)
pub trait OwnedCell<K: Value + 'static>: Sized {
    fn parse_cell(text: &str) -> Result<Self, String>;
    fn parse_elem(text: &str) -> Result<Self, String>;
    fn as_arg(&self) -> <K as Value>::SelfType<'_>;
}

fn unescape(text: &str) -> Result<String, String> {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('0') => out.push('\0'),
            Some('\\') => out.push('\\'),
            Some('\'') => out.push('\''),
            Some('"') => out.push('"'),
            Some('u') => {
                if chars.next() != Some('{') {
                    return Err("invalid \\u escape".to_string());
                }
                let mut hex = String::new();
                let mut closed = false;
                for h in chars.by_ref() {
                    if h == '}' {
                        closed = true;
                        break;
                    }
                    hex.push(h);
                    if hex.len() > 6 {
                        return Err("\\u escape too long".to_string());
                    }
                }
                if !closed {
                    return Err("unterminated \\u escape".to_string());
                }
                let cp =
                    u32::from_str_radix(&hex, 16).map_err(|_| "invalid \\u escape".to_string())?;
                out.push(char::from_u32(cp).ok_or("invalid codepoint in \\u escape")?);
            }
            other => return Err(format!("unknown escape {other:?}")),
        }
    }
    Ok(out)
}

/// Splits "a, b, c" on top-level commas, respecting brackets and quotes.
fn split_top_level(text: &str) -> Result<Vec<String>, String> {
    let mut elems = Vec::new();
    let mut depth = 0i32;
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    let mut cur = String::new();
    for c in text.chars() {
        if escaped {
            cur.push(c);
            escaped = false;
            continue;
        }
        if (in_single || in_double) && c == '\\' {
            cur.push(c);
            escaped = true;
            continue;
        }
        match c {
            '"' if !in_single => {
                in_double = !in_double;
                cur.push(c);
            }
            '\'' if !in_double => {
                in_single = !in_single;
                cur.push(c);
            }
            '(' | '[' if !in_single && !in_double => {
                depth += 1;
                cur.push(c);
            }
            ')' | ']' if !in_single && !in_double => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 && !in_single && !in_double => {
                elems.push(cur.clone());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    if in_single || in_double || escaped {
        return Err("unbalanced quotes".to_string());
    }
    elems.push(cur);
    Ok(elems)
}

fn parse_char_text(text: &str) -> Result<char, String> {
    let t = text.trim();
    let inner = t
        .strip_prefix('\'')
        .and_then(|x| x.strip_suffix('\''))
        .ok_or_else(|| format!("expected char like 'a', got '{t}'"))?;
    let s = unescape(inner)?;
    let mut it = s.chars();
    let c = it.next().ok_or("empty char")?;
    if it.next().is_some() {
        return Err(format!("expected a single char, got '{inner}'"));
    }
    Ok(c)
}

fn parse_quoted_str(text: &str) -> Result<String, String> {
    let t = text.trim();
    let inner = t
        .strip_prefix('"')
        .and_then(|x| x.strip_suffix('"'))
        .ok_or_else(|| format!("expected quoted string like \"abc\", got '{t}'"))?;
    unescape(inner)
}

macro_rules! impl_owned_scalar {
    ($($t:ty),*) => {
        $(
            impl OwnedCell<$t> for $t {
                fn parse_cell(text: &str) -> Result<Self, String> {
                    text.trim()
                        .parse::<$t>()
                        .map_err(|_| format!("expected {}, got '{}'", type_name_of::<$t>(), text.trim()))
                }
                fn parse_elem(text: &str) -> Result<Self, String> {
                    Self::parse_cell(text)
                }
                fn as_arg(&self) -> <$t as Value>::SelfType<'_> {
                    *self
                }
            }
        )*
    };
}

impl_owned_scalar!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64, bool);

impl OwnedCell<()> for () {
    fn parse_cell(text: &str) -> Result<Self, String> {
        if text.trim() == "()" {
            Ok(())
        } else {
            Err(format!("expected '()', got '{}'", text.trim()))
        }
    }
    fn parse_elem(text: &str) -> Result<Self, String> {
        Self::parse_cell(text)
    }
    fn as_arg(&self) -> <() as Value>::SelfType<'_> {}
}

impl OwnedCell<char> for char {
    fn parse_cell(text: &str) -> Result<Self, String> {
        parse_char_text(text)
    }
    fn parse_elem(text: &str) -> Result<Self, String> {
        Self::parse_cell(text)
    }
    fn as_arg(&self) -> <char as Value>::SelfType<'_> {
        *self
    }
}

impl OwnedCell<&'static str> for String {
    fn parse_cell(text: &str) -> Result<Self, String> {
        Ok(text.to_string())
    }
    fn parse_elem(text: &str) -> Result<Self, String> {
        parse_quoted_str(text)
    }
    fn as_arg(&self) -> <&'static str as Value>::SelfType<'_> {
        self.as_str()
    }
}

impl OwnedCell<String> for String {
    fn parse_cell(text: &str) -> Result<Self, String> {
        Ok(text.to_string())
    }
    fn parse_elem(text: &str) -> Result<Self, String> {
        parse_quoted_str(text)
    }
    fn as_arg(&self) -> <String as Value>::SelfType<'_> {
        self.clone()
    }
}

fn parse_hex_cell(text: &str) -> Result<Vec<u8>, String> {
    let t = text.trim();
    let hex = t
        .strip_prefix("0x")
        .ok_or_else(|| format!("expected 0x-prefixed hex, got '{t}'"))?;
    if hex.len() % 2 != 0 {
        return Err(format!("odd-length hex value '{hex}'"));
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|_| format!("invalid hex byte '{}'", &hex[i..i + 2]))
        })
        .collect()
}

fn parse_byte_list(text: &str) -> Result<Vec<u8>, String> {
    let t = text.trim();
    let inner = t
        .strip_prefix('[')
        .and_then(|x| x.strip_suffix(']'))
        .ok_or_else(|| format!("expected byte list like [1, 2], got '{t}'"))?;
    if inner.trim().is_empty() {
        return Ok(Vec::new());
    }
    inner
        .split(',')
        .map(|part| {
            part.trim()
                .parse::<u8>()
                .map_err(|_| format!("invalid byte '{part}'"))
        })
        .collect()
}

impl OwnedCell<&'static [u8]> for Vec<u8> {
    fn parse_cell(text: &str) -> Result<Self, String> {
        parse_hex_cell(text)
    }
    fn parse_elem(text: &str) -> Result<Self, String> {
        parse_byte_list(text)
    }
    fn as_arg(&self) -> <&'static [u8] as Value>::SelfType<'_> {
        self.as_slice()
    }
}

macro_rules! impl_owned_tuple2 {
    ($k:ty, $o:ty, |$s:ident| $arg:expr) => {
        impl OwnedCell<($k, $k)> for ($o, $o) {
            fn parse_cell(text: &str) -> Result<Self, String> {
                let inner = text
                    .trim()
                    .strip_prefix('(')
                    .and_then(|x| x.strip_suffix(')'))
                    .ok_or_else(|| format!("expected tuple like (a, b), got '{}'", text.trim()))?;
                let elems = split_top_level(inner)?;
                if elems.len() != 2 {
                    return Err(format!(
                        "expected 2 tuple elements, got {} in '{text}'",
                        elems.len()
                    ));
                }
                Ok((
                    <$o as OwnedCell<$k>>::parse_elem(elems[0].trim())?,
                    <$o as OwnedCell<$k>>::parse_elem(elems[1].trim())?,
                ))
            }
            fn parse_elem(text: &str) -> Result<Self, String> {
                <($o, $o) as OwnedCell<($k, $k)>>::parse_cell(text)
            }
            fn as_arg(&self) -> <($k, $k) as Value>::SelfType<'_> {
                let $s = self;
                $arg
            }
        }
    };
}

macro_rules! impl_owned_tuple3 {
    ($k:ty, $o:ty, |$s:ident| $arg:expr) => {
        impl OwnedCell<($k, $k, $k)> for ($o, $o, $o) {
            fn parse_cell(text: &str) -> Result<Self, String> {
                let inner = text
                    .trim()
                    .strip_prefix('(')
                    .and_then(|x| x.strip_suffix(')'))
                    .ok_or_else(|| {
                        format!("expected tuple like (a, b, c), got '{}'", text.trim())
                    })?;
                let elems = split_top_level(inner)?;
                if elems.len() != 3 {
                    return Err(format!(
                        "expected 3 tuple elements, got {} in '{text}'",
                        elems.len()
                    ));
                }
                Ok((
                    <$o as OwnedCell<$k>>::parse_elem(elems[0].trim())?,
                    <$o as OwnedCell<$k>>::parse_elem(elems[1].trim())?,
                    <$o as OwnedCell<$k>>::parse_elem(elems[2].trim())?,
                ))
            }
            fn parse_elem(text: &str) -> Result<Self, String> {
                <($o, $o, $o) as OwnedCell<($k, $k, $k)>>::parse_cell(text)
            }
            fn as_arg(&self) -> <($k, $k, $k) as Value>::SelfType<'_> {
                let $s = self;
                $arg
            }
        }
    };
}

macro_rules! impl_owned_tuples {
    ($k:ty, $o:ty, |$s2:ident| $a2:expr, |$s3:ident| $a3:expr) => {
        impl_owned_tuple2!($k, $o, |$s2| $a2);
        impl_owned_tuple3!($k, $o, |$s3| $a3);
    };
}

impl_owned_tuples!(u8, u8, |s| (s.0, s.1), |s| (s.0, s.1, s.2));
impl_owned_tuples!(u16, u16, |s| (s.0, s.1), |s| (s.0, s.1, s.2));
impl_owned_tuples!(u32, u32, |s| (s.0, s.1), |s| (s.0, s.1, s.2));
impl_owned_tuples!(u64, u64, |s| (s.0, s.1), |s| (s.0, s.1, s.2));
impl_owned_tuples!(u128, u128, |s| (s.0, s.1), |s| (s.0, s.1, s.2));
impl_owned_tuples!(i8, i8, |s| (s.0, s.1), |s| (s.0, s.1, s.2));
impl_owned_tuples!(i16, i16, |s| (s.0, s.1), |s| (s.0, s.1, s.2));
impl_owned_tuples!(i32, i32, |s| (s.0, s.1), |s| (s.0, s.1, s.2));
impl_owned_tuples!(i64, i64, |s| (s.0, s.1), |s| (s.0, s.1, s.2));
impl_owned_tuples!(i128, i128, |s| (s.0, s.1), |s| (s.0, s.1, s.2));
impl_owned_tuples!(bool, bool, |s| (s.0, s.1), |s| (s.0, s.1, s.2));
impl_owned_tuples!(char, char, |s| (s.0, s.1), |s| (s.0, s.1, s.2));
impl_owned_tuples!((), (), |s| (s.0, s.1), |s| (s.0, s.1, s.2));
impl_owned_tuples!(
    &'static str,
    String,
    |s| (s.0.as_str(), s.1.as_str()),
    |s| (s.0.as_str(), s.1.as_str(), s.2.as_str())
);
impl_owned_tuples!(String, String, |s| (s.0.clone(), s.1.clone()), |s| (
    s.0.clone(),
    s.1.clone(),
    s.2.clone()
));
impl_owned_tuples!(
    &'static [u8],
    Vec<u8>,
    |s| (s.0.as_slice(), s.1.as_slice()),
    |s| (s.0.as_slice(), s.1.as_slice(), s.2.as_slice())
);

// ---------------------------------------------------------------------------
// Typed read/write dispatch over the registered type combinations
// ---------------------------------------------------------------------------

macro_rules! registry_keys {
    ($values:ident) => {
        $values!(u8, u8);
        $values!(u16, u16);
        $values!(u32, u32);
        $values!(u64, u64);
        $values!(u128, u128);
        $values!(i8, i8);
        $values!(i16, i16);
        $values!(i32, i32);
        $values!(i64, i64);
        $values!(i128, i128);
        $values!(bool, bool);
        $values!(char, char);
        $values!((), ());
        $values!(&str, String);
        $values!(String, String);
        $values!(&[u8], Vec<u8>);
        $values!((u8, u8), (u8, u8));
        $values!((u16, u16), (u16, u16));
        $values!((u32, u32), (u32, u32));
        $values!((u64, u64), (u64, u64));
        $values!((u128, u128), (u128, u128));
        $values!((i8, i8), (i8, i8));
        $values!((i16, i16), (i16, i16));
        $values!((i32, i32), (i32, i32));
        $values!((i64, i64), (i64, i64));
        $values!((i128, i128), (i128, i128));
        $values!((bool, bool), (bool, bool));
        $values!((char, char), (char, char));
        $values!(((), ()), ((), ()));
        $values!((&str, &str), (String, String));
        $values!((String, String), (String, String));
        $values!((&[u8], &[u8]), (Vec<u8>, Vec<u8>));
        $values!((u8, u8, u8), (u8, u8, u8));
        $values!((u16, u16, u16), (u16, u16, u16));
        $values!((u32, u32, u32), (u32, u32, u32));
        $values!((u64, u64, u64), (u64, u64, u64));
        $values!((u128, u128, u128), (u128, u128, u128));
        $values!((i8, i8, i8), (i8, i8, i8));
        $values!((i16, i16, i16), (i16, i16, i16));
        $values!((i32, i32, i32), (i32, i32, i32));
        $values!((i64, i64, i64), (i64, i64, i64));
        $values!((i128, i128, i128), (i128, i128, i128));
        $values!((bool, bool, bool), (bool, bool, bool));
        $values!((char, char, char), (char, char, char));
        $values!(((), (), ()), ((), (), ()));
        $values!((&str, &str, &str), (String, String, String));
        $values!((String, String, String), (String, String, String));
        $values!((&[u8], &[u8], &[u8]), (Vec<u8>, Vec<u8>, Vec<u8>));
    };
}

macro_rules! registry_values {
    ($pair:ident, $kt:ty, $ko:ty) => {
        $pair!($kt, u8, $ko, u8);
        $pair!($kt, u16, $ko, u16);
        $pair!($kt, u32, $ko, u32);
        $pair!($kt, u64, $ko, u64);
        $pair!($kt, u128, $ko, u128);
        $pair!($kt, i8, $ko, i8);
        $pair!($kt, i16, $ko, i16);
        $pair!($kt, i32, $ko, i32);
        $pair!($kt, i64, $ko, i64);
        $pair!($kt, i128, $ko, i128);
        $pair!($kt, f32, $ko, f32);
        $pair!($kt, f64, $ko, f64);
        $pair!($kt, bool, $ko, bool);
        $pair!($kt, char, $ko, char);
        $pair!($kt, (), $ko, ());
        $pair!($kt, &str, $ko, String);
        $pair!($kt, String, $ko, String);
        $pair!($kt, &[u8], $ko, Vec<u8>);
    };
}

fn read_rows<K, V>(txn: &ReadTransaction, name: &str) -> Result<Vec<(String, String)>, redb::Error>
where
    K: Key + 'static + Borrow<<K as Value>::SelfType<'static>>,
    V: Value + 'static,
    for<'a> K::SelfType<'a>: FmtCell,
    for<'a> V::SelfType<'a>: FmtCell,
{
    let definition: TableDefinition<'_, K, V> = TableDefinition::new(name);
    let table = txn.open_table(definition)?;
    let mut iter = table.range::<K>(..)?;
    let mut rows = Vec::new();
    while let Some((key, value)) = iter.next().transpose()? {
        rows.push((key.value().fmt_cell(), value.value().fmt_cell()));
    }
    Ok(rows)
}

/// Reads a table with the types described by `desc`, which is obtained by
/// probing the table definition. Only type combinations covered by the
/// registry below can be read; extend `registry_keys!` / `registry_values!`
/// to support more.
pub fn read_table_dyn(
    txn: &ReadTransaction,
    name: &str,
    desc: &TableTypeDesc,
) -> Result<Vec<(String, String)>, String> {
    let key_name = desc.key.as_str();
    let value_name = desc.value.as_str();

    macro_rules! read_pair {
        ($kt:ty, $vt:ty, $ko:ty, $vo:ty) => {
            if key_name == type_name_of::<$kt>() && value_name == type_name_of::<$vt>() {
                return read_rows::<$kt, $vt>(txn, name).map_err(|e| e.to_string());
            }
        };
    }
    macro_rules! read_values {
        ($kt:ty, $ko:ty) => {
            registry_values!(read_pair, $kt, $ko);
        };
    }
    registry_keys!(read_values);

    Err(format!(
        "table '{name}' has unsupported key/value types Table<{key_name}, {value_name}>"
    ))
}

fn write_rows<K, V, KO, VO>(
    txn: &WriteTransaction,
    name: &str,
    rows: Vec<(String, String)>,
) -> Result<(), String>
where
    K: Key + 'static + Borrow<<K as Value>::SelfType<'static>>,
    V: Value + 'static,
    KO: OwnedCell<K>,
    VO: OwnedCell<V>,
    for<'a> K::SelfType<'a>: FmtCell,
{
    // Parse everything first: a single invalid cell aborts the whole edit
    // before the table is touched.
    let mut parsed = Vec::with_capacity(rows.len());
    for (key_text, value_text) in &rows {
        let key = KO::parse_cell(key_text).map_err(|e| format!("invalid key '{key_text}': {e}"))?;
        let value = VO::parse_cell(value_text)
            .map_err(|e| format!("invalid value for key '{key_text}': {e}"))?;
        parsed.push((key, value));
    }

    let definition: TableDefinition<'_, K, V> = TableDefinition::new(name);
    let mut table = txn
        .open_table(definition)
        .map_err(|e| format!("open table '{name}' failed: {e}"))?;

    // Remove all existing entries (format key -> parse back is an exact
    // round-trip for every registered type).
    {
        let existing: Vec<String> = {
            let mut iter = table
                .range::<K>(..)
                .map_err(|e| format!("read table '{name}' failed: {e}"))?;
            let mut keys = Vec::new();
            while let Some((key, _)) = iter.next().transpose().map_err(|e| e.to_string())? {
                keys.push(key.value().fmt_cell());
            }
            keys
        };
        for key_text in existing {
            let key = KO::parse_cell(&key_text)
                .map_err(|e| format!("failed to re-parse existing key '{key_text}': {e}"))?;
            table
                .remove(key.as_arg())
                .map_err(|e| format!("remove key '{key_text}' failed: {e}"))?;
        }
    }

    for (key, value) in parsed {
        table
            .insert(key.as_arg(), value.as_arg())
            .map_err(|e| format!("insert failed: {e}"))?;
    }
    Ok(())
}

/// Replaces the entire content of table `name` with `rows`, which are
/// edited (key, value) cell strings as produced by `read_table_dyn`.
/// All rows are validated before anything is written; on any error the
/// transaction is left uncommitted and the table stays unchanged.
pub fn write_table_dyn(
    txn: &WriteTransaction,
    name: &str,
    desc: &TableTypeDesc,
    rows: Vec<(String, String)>,
) -> Result<(), String> {
    let key_name = desc.key.as_str();
    let value_name = desc.value.as_str();

    macro_rules! write_pair {
        ($kt:ty, $vt:ty, $ko:ty, $vo:ty) => {
            if key_name == type_name_of::<$kt>() && value_name == type_name_of::<$vt>() {
                return write_rows::<$kt, $vt, $ko, $vo>(txn, name, rows);
            }
        };
    }
    macro_rules! write_values {
        ($kt:ty, $ko:ty) => {
            registry_values!(write_pair, $kt, $ko);
        };
    }
    registry_keys!(write_values);

    Err(format!(
        "table '{name}' has key/value types Table<{key_name}, {value_name}> that cannot be edited (unsupported)"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_encoding() {
        assert_eq!(bytes_to_hex(&[]), "0x");
        assert_eq!(bytes_to_hex(&[0x01, 0xab, 0xff]), "0x01abff");
    }

    #[test]
    fn fmt_cell_variants() {
        assert_eq!(42u64.fmt_cell(), "42");
        assert_eq!((-7i32).fmt_cell(), "-7");
        assert_eq!(().fmt_cell(), "()");
        assert_eq!("hi".fmt_cell(), "hi");
        assert_eq!(String::from("hi").fmt_cell(), "hi");
        assert_eq!((1u128, 2u128).fmt_cell(), "(1, 2)");
        assert_eq!((&[0u8, 255][..]).fmt_cell(), "0x00ff");
    }

    #[test]
    fn type_names_match_redb_builtin() {
        assert_eq!(type_name_of::<&str>(), "&str");
        assert_eq!(type_name_of::<u64>(), "u64");
        assert_eq!(type_name_of::<(u128, u128)>(), "(u128,u128)");
        assert_eq!(type_name_of::<&[u8]>(), "&[u8]");
    }

    #[test]
    fn parse_scalars() {
        use OwnedCell as OC;
        assert_eq!(<u64 as OC<u64>>::parse_cell("42").unwrap(), 42);
        assert_eq!(<i32 as OC<i32>>::parse_cell(" -7 ").unwrap(), -7);
        assert_eq!(
            <u128 as OC<u128>>::parse_cell("340282366920938463463374607431768211455").unwrap(),
            u128::MAX
        );
        assert!(<u64 as OC<u64>>::parse_cell("abc").is_err());
        assert!(<bool as OC<bool>>::parse_cell("yes").is_err());
        assert!(<bool as OC<bool>>::parse_cell("true").unwrap());
        assert_eq!(<f64 as OC<f64>>::parse_cell("3.5").unwrap(), 3.5);
        assert_eq!(<() as OC<()>>::parse_cell("()").unwrap(), ());
        assert!(<() as OC<()>>::parse_cell("( )").is_err());
    }

    #[test]
    fn parse_chars() {
        use OwnedCell as OC;
        assert_eq!(<char as OC<char>>::parse_cell("'a'").unwrap(), 'a');
        assert_eq!(<char as OC<char>>::parse_cell("'\\n'").unwrap(), '\n');
        assert_eq!(<char as OC<char>>::parse_cell("'\\''").unwrap(), '\'');
        assert_eq!(<char as OC<char>>::parse_cell("'\\u{41}'").unwrap(), 'A');
        assert!(<char as OC<char>>::parse_cell("a").is_err());
        assert!(<char as OC<char>>::parse_cell("'ab'").is_err());
    }

    #[test]
    fn parse_str_raw_and_quoted() {
        use OwnedCell as OC;
        // standalone &str cells are raw text
        assert_eq!(
            <String as OC<&'static str>>::parse_cell("hello, world").unwrap(),
            "hello, world"
        );
        // inside tuples they are Debug-quoted
        assert_eq!(
            <String as OC<&'static str>>::parse_elem("\"a, b\"").unwrap(),
            "a, b"
        );
        assert_eq!(
            <String as OC<&'static str>>::parse_elem("\"line\\nbreak\"").unwrap(),
            "line\nbreak"
        );
        assert!(<String as OC<&'static str>>::parse_elem("no quotes").is_err());
    }

    #[test]
    fn parse_bytes_hex_and_list() {
        use OwnedCell as OC;
        assert_eq!(
            <Vec<u8> as OC<&'static [u8]>>::parse_cell("0x01abff").unwrap(),
            vec![1, 0xab, 0xff]
        );
        assert_eq!(
            <Vec<u8> as OC<&'static [u8]>>::parse_cell("0x").unwrap(),
            Vec::<u8>::new()
        );
        assert!(<Vec<u8> as OC<&'static [u8]>>::parse_cell("01ab").is_err());
        assert!(<Vec<u8> as OC<&'static [u8]>>::parse_cell("0xzz").is_err());
        assert_eq!(
            <Vec<u8> as OC<&'static [u8]>>::parse_elem("[1, 2, 255]").unwrap(),
            vec![1, 2, 255]
        );
        assert_eq!(
            <Vec<u8> as OC<&'static [u8]>>::parse_elem("[]").unwrap(),
            Vec::<u8>::new()
        );
        assert!(<Vec<u8> as OC<&'static [u8]>>::parse_elem("[300]").is_err());
    }

    #[test]
    fn parse_tuples() {
        use OwnedCell as OC;
        let pair: (u64, u64) = OC::<(u64, u64)>::parse_cell("(1, 2)").unwrap();
        assert_eq!(pair, (1, 2));
        let pair: (u128, u128) = OC::<(u128, u128)>::parse_cell("(1, 2)").unwrap();
        assert_eq!(pair, (1, 2));
        let triple: (i32, i32, i32) = OC::<(i32, i32, i32)>::parse_cell("(-1, 2, 3)").unwrap();
        assert_eq!(triple, (-1, 2, 3));
        let sp: (String, String) =
            OC::<(&'static str, &'static str)>::parse_cell(r#"("a, b", "c")"#).unwrap();
        assert_eq!(sp, ("a, b".to_string(), "c".to_string()));
        let bp: (Vec<u8>, Vec<u8>) =
            OC::<(&'static [u8], &'static [u8])>::parse_cell("([1, 2], [3])").unwrap();
        assert_eq!(bp, (vec![1, 2], vec![3]));
        let cp: (char, char) = OC::<(char, char)>::parse_cell("('a', '\\n')").unwrap();
        assert_eq!(cp, ('a', '\n'));
        assert!(<(u64, u64) as OC<(u64, u64)>>::parse_cell("(1)").is_err());
        assert!(<(u64, u64) as OC<(u64, u64)>>::parse_cell("(1, 2, 3)").is_err());
        assert!(<(u64, u64) as OC<(u64, u64)>>::parse_cell("1, 2").is_err());
        assert!(<(u64, u64) as OC<(u64, u64)>>::parse_cell("(1, x)").is_err());
    }

    #[test]
    fn split_respects_quotes_and_nesting() {
        let parts = split_top_level("\"a, b\", 'c', [1, 2], (3, 4)").unwrap();
        assert_eq!(parts, vec!["\"a, b\"", " 'c'", " [1, 2]", " (3, 4)"]);
        assert!(split_top_level("\"unterminated").is_err());
    }

    #[test]
    fn fmt_parse_roundtrip_per_type() {
        use OwnedCell as OC;
        // keys
        assert_eq!(<u64 as OC<u64>>::parse_cell(&42u64.fmt_cell()).unwrap(), 42);
        assert_eq!(<i8 as OC<i8>>::parse_cell(&(-7i8).fmt_cell()).unwrap(), -7);
        assert_eq!(
            <char as OC<char>>::parse_cell(&'\n'.fmt_cell()).unwrap(),
            '\n'
        );
        assert_eq!(
            <String as OC<&'static str>>::parse_cell(&"a,b (c) 'd'".fmt_cell()).unwrap(),
            "a,b (c) 'd'"
        );
        let hex = bytes_to_hex(&[0xde, 0xad]);
        assert_eq!(
            <Vec<u8> as OC<&'static [u8]>>::parse_cell(&hex).unwrap(),
            vec![0xde, 0xad]
        );
        // tuples roundtrip through fmt_cell + parse_cell
        let t2 = (340282366920938463463374607431768211455u128, 7u128);
        let parsed2 = <(u128, u128) as OC<(u128, u128)>>::parse_cell(&t2.fmt_cell()).unwrap();
        assert_eq!(parsed2, t2);
        let ts = ("x, y".to_string(), "z".to_string());
        let parsed_s =
            <(String, String) as OC<(&'static str, &'static str)>>::parse_cell(&ts.fmt_cell())
                .unwrap();
        assert_eq!(parsed_s, ts);
        let tb = (vec![1u8, 2], vec![255]);
        let parsed_b =
            <(Vec<u8>, Vec<u8>) as OC<(&'static [u8], &'static [u8])>>::parse_cell(&tb.fmt_cell())
                .unwrap();
        assert_eq!(parsed_b, tb);
        let t3 = (1u64, 2, 3);
        let parsed3 = <(u64, u64, u64) as OC<(u64, u64, u64)>>::parse_cell(&t3.fmt_cell()).unwrap();
        assert_eq!(parsed3, t3);
        // float values roundtrip (shortest Debug repr reparses identically)
        assert_eq!(
            <f64 as OC<f64>>::parse_cell(&std::f64::consts::PI.fmt_cell()).unwrap(),
            std::f64::consts::PI
        );
    }
}
