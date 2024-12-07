# Rust - redb cli
RedbCLI is a command-line tool for managing and operating Redb databases. It provides various commands to create, delete, query, and edit database tables.


## Requires
Need vim

## Features
- Set database path
- Use specific tables
- Edit table data
- Query table information
- Create and delete tables

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
5. Edit table data:
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