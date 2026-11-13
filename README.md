# Rust - redb cli
RedbCLI is a command-line tool for managing and operating Redb databases. It provides various commands to create, delete, query, and edit database tables.


## Requires
Need vim

## Features
- Set database path
- Use specific tables
- Browse tables of any key/value type (integers, tuples such as `Table<(u128,u128), u64>`, strings, byte slices, etc.)
- Edit table data of any supported type (values are shown and edited as typed strings: decimals, `(a, b)` tuples, `0x` hex for bytes)
- Query table information, including each table's key/value types
- Create and delete tables

## Supported types

redb only exposes typed table access (`TableDefinition<K, V>`), so redbcli has to
instantiate a reader/writer per `(K, V)` pair. The matrix is deliberately bounded
— every entry costs compile time and binary size — and it is enforced by the
error message you get for anything outside it:

| Key | Value |
| --- | --- |
| `u8`..`u128`, `i8`..`i128`, `bool`, `char`, `()`, `&str`, `String`, `&[u8]` | any of the same scalar/string/byte types (values may also be `f32`/`f64`), or a homogeneous 4-6 element tuple of the non-float ones, e.g. `(u128, u128, u128, u128, u128)` |
| 2-3 element homogeneous tuples of the non-float types | scalars/strings/bytes (no floats, no tuple values) |

Not supported yet: float keys, tuple keys of 4+ elements, 2-3 element tuple
values under non-tuple keys (they are only registered from arity 4 up), 7+
element tuple values, tuple values under tuple keys, and heterogeneous or nested
tuples. Issue #4 tracks the
design; `docs/` is the place to look before extending the registries.

How values are spelled — one form per type, and every printed cell can be pasted
back:

- a byte value is `0x` hex everywhere, as a whole value (`0x6161`) and as a tuple
  element (`(0x6161, 0x6262)`); the decimal list form (`[97, 97]`) is still
  accepted when editing, so cells written by older versions keep parsing;
- a string is bare as a whole value (`hello`) and quoted inside a tuple
  (`("a,b", "c")`), where the quotes are what keeps a comma inside one element.
  Both forms are accepted in both places; what is removed is the *outer quote
  delimiter*: pasting `"hello"` into a scalar string column stores `hello`,
  while quotes inside bare text (`a"b`, `don't`) stay part of the value. A
  stored value that itself begins and ends with `"` is printed quoted and
  escaped (`"\"hello\""`) so that saving the table without edits cannot
  change it.

## Installation

1. Ensure you have Rust and Cargo installed.
2. Clone the project repository:
```shell
git clone https://github.com/jokemanfire/redbcli
cd redbcli
```
3. Build the project:
```shell
cargo build --release
```
4. Add the generated executable to your PATH:
```shell
cp target/release/redbcli /usr/local/bin/
```

## Usage
1. Start RedbCLI:
``` sh
redbcli
``` 
2. Set the database path:
``` sh
set /path/to/your/database
``` 
3. Use a specific table:
``` sh
use your_table_name
```
4. Query table information:
``` sh
info tables
info key your_key
info table your_table_name
```
5. Edit table data (whole table is dumped as JSON, keys and values are typed strings; removing a JSON entry deletes the row, an invalid value aborts the edit without changing anything):
```sh
edit
```
6. Create a new table:
```sh
create your_table_name
```
7. Delete a table:
```sh
delete your_table_name
```
8. Exit the program:
```sh
exit
```
## Command List
* set <filepath>: Set the database path.
* use <tablename>: Use a specific table.
* edit: Edit the data of the current table.
* info [tables | key <key> | table <tablename>]: Query table information.
* create <tablename>: Create a new table.
* delete <tablename>: Delete a table.
* exit: Exit the program.

## Example

```shell
$ redbcli
DB:[/path/to/your/database] TAB:[] 
>> set /path/to/your/database
-> set database success! 

DB:[/path/to/your/database] TAB:[] 
>> use my_table
-> Use table my_table

DB:[/path/to/your/database] TAB:[my_table] 
>> info tables
+------------------+
|      Tables      |
+------------------+
|     my_table     |
+------------------+

DB:[/path/to/your/database] TAB:[my_table] 
>> info key my_key
-> data 
{
  "my_key": "my_value"
}

DB:[/path/to/your/database] TAB:[my_table] 
>> edit
-> Save data to update the database

DB:[/path/to/your/database] TAB:[my_table] 
>> create new_table
-> Table created successfully

DB:[/path/to/your/database] TAB:[my_table] 
>> delete new_table
-> Table deleted successfully

DB:[/path/to/your/database] TAB:[my_table] 
>> exit
-> Exiting ... 
```

## License
This project is licensed under the MIT License. See the LICENSE file for details.


## Contributing
Contributions are welcome! Please open an issue or submit a pull request if you have any improvements or bug fixes.

For more detailed information, please refer to the source code and documentation.