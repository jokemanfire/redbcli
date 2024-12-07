# Rust - redb cli
Command line program to operate redb, and provide a command-line interface for the entire redb operation.


## Requires
Need vim

## Features
- Open a redb database.
- Info will show the tables and datas.
- Edit the redb database.
- Create and Delete a table(TODO).

## Example

```shell
example:

DB:[] TAB:[] 
>> set /home/10346053@zte.intra/hdy/github/redbcli/example/stores.redb
-> set database success! 

DB:[/home/10346053@zte.intra/hdy/github/redbcli/example/stores.redb] TAB:[] 
>> use images
-> Use table images 

DB:[/home/10346053@zte.intra/hdy/github/redbcli/example/stores.redb] TAB:[images] 
>> edit
Save data to update the database

DB:[/home/10346053@zte.intra/hdy/github/redbcli/example/stores.redb] TAB:[images] 
>> edit
No changed!


```

## License
This project is licensed under the MIT License. See the LICENSE file for details.


## Contributing
Contributions are welcome! Please open an issue or submit a pull request if you have any improvements or bug fixes.

For more detailed information, please refer to the source code and documentation.