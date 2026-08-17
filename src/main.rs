use clap::Parser;
use redb::Database;
use redbcli::flags::{Binflags, Cli};
use redbcli::pretty_print::PrintTable;
use redbcli::redbcontrol::DealTable;
use redbcli::{
    flags::{Commands, InfoCommands},
    redbcontrol::{CommonDbManager, DealData},
};
use redbcli::{write_io_error, write_io_success, KvInfo, TableInfo, TableMeta};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::io::Write;
use std::path::PathBuf;
#[derive(Default)]
pub struct CliStatus {
    tablename: String,
    filepath: String,
    dbm: CommonDbManager,
}

fn main() -> Result<(), String> {
    let mut clistatus = CliStatus::default();
    let parse_flags = Binflags::parse();
    if let Some(db_path) = parse_flags.path {
        clistatus.filepath = db_path;
    }
    let history_path = PathBuf::from("/tmp/redbcli");
    if !history_path.exists() {
        std::fs::create_dir_all(&history_path).expect("create history dir failed");
    }
    let file_history = history_path.join("history.txt");
    //check history file
    {
        if !file_history.exists() {
            std::fs::File::create(&file_history).expect("create history file failed");
        }
    }

    let mut rl = DefaultEditor::new().unwrap();
    if rl.load_history(&file_history).is_err() {
        println!("No previous history.");
    }
    if !clistatus.filepath.is_empty() {
        clistatus
            .dbm
            .setdbpath(clistatus.filepath.clone())
            .map_err(|e| e.to_string())?;
    }
    if !clistatus.tablename.is_empty() {
        clistatus
            .dbm
            .settablename(clistatus.tablename.clone())
            .map_err(|e| e.to_string())?;
    }

    loop {
        let prompt = format!(
            "\nDB:[{}] TAB:[{}] \n>> ",
            clistatus.filepath, clistatus.tablename
        );
        let readline = rl.readline(&prompt);

        match readline {
            Ok(line) => {
                let _ = rl.add_history_entry(line.as_str());
                match respond(&line, &mut clistatus) {
                    Ok(quit) => {
                        if quit {
                            break;
                        }
                    }
                    Err(err) => {
                        write_io_error(err)?;
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    let _ = rl.save_history(&file_history);
    Ok(())
}

fn respond(line: &str, status: &mut CliStatus) -> Result<bool, String> {
    let args = shlex::split(line).ok_or("error: Invalid quoting")?;
    let cli = Cli::try_parse_from(args).map_err(|e| e.to_string())?;

    match cli.command {
        Commands::Set { filepath } => {
            write_io_success("set database success!".to_string())?;
            status.filepath = filepath;
            status
                .dbm
                .setdbpath(status.filepath.clone())
                .map_err(|e| e.to_string())?;
            Ok(false)
        }

        Commands::Use { tablename } => {
            if status.filepath.is_empty() {
                return Err("you must set file path first !!".to_string());
            }
            status.tablename = tablename.clone();
            status
                .dbm
                .settablename(tablename.clone())
                .map_err(|e| e.to_string())?;
            write_io_success(format!("Use table {}", tablename))?;
            Ok(false)
        }
        Commands::Edit => {
            if status.tablename.is_empty() {
                return Err("you must set table first !!".to_string());
            }
            let (_desc, rows) = status
                .dbm
                .get_all_dyn(&status.tablename)
                .map_err(|e| e.to_string())?;
            let json_data = redbcli::rows_to_json(&rows)?;

            let mut temp_file = tempfile::NamedTempFile::new().map_err(|e| e.to_string())?;
            temp_file
                .write_all(json_data.as_bytes())
                .map_err(|e| e.to_string())?;

            let temp_path = temp_file
                .path()
                .to_str()
                .ok_or("Invalid path".to_string())?;

            let mut child = std::process::Command::new("vim")
                .arg(temp_path)
                .arg("+syntax on")
                .arg("+set number")
                .arg("+set filetype=json")
                .stdin(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .spawn()
                .map_err(|e| e.to_string())?;

            let vim_status = child.wait().map_err(|e| e.to_string())?;

            if !vim_status.success() {
                return Err("Vim exited with an error".to_string());
            }

            let modified_data = std::fs::read_to_string(temp_path).map_err(|e| e.to_string())?;
            if modified_data == json_data {
                println!("No changed!");
                return Ok(false);
            }
            let new_rows = redbcli::json_to_rows(&modified_data)?;
            status
                .dbm
                .update_all_dyn(&status.tablename, new_rows)
                .map_err(|e| e.to_string())?;
            write_io_success("Save data to update the database".to_string())?;
            Ok(false)
        }

        Commands::Info(subcmd) => {
            let sub_cmd = subcmd.command.unwrap_or(InfoCommands::Tables);
            match sub_cmd {
                InfoCommands::Tables => {
                    let result = status.dbm.list_table_types().map_err(|e| e.to_string())?;
                    let tables = result
                        .into_iter()
                        .map(|(name, desc)| TableMeta {
                            name,
                            key_type: desc.key,
                            value_type: desc.value,
                        })
                        .collect();
                    TableInfo { tables }.print_data();
                    Ok(false)
                }
                InfoCommands::Key { key } => {
                    if status.tablename.is_empty() {
                        return Err("you must use table to select !!".to_string());
                    }
                    let result = status
                        .dbm
                        .get_by_key(key.clone())
                        .map_err(|e| e.to_string())?;
                    let kvdatas = vec![(key, result)];
                    KvInfo {
                        kvdatas,
                        key_type: "&str".to_string(),
                        value_type: "&str".to_string(),
                    }
                    .print_data();
                    Ok(false)
                }
                InfoCommands::Table { tablename } => {
                    let (desc, rows) = status
                        .dbm
                        .get_all_dyn(&tablename)
                        .map_err(|e| e.to_string())?;
                    status.tablename = tablename.clone();
                    status.dbm.tablename = tablename;
                    KvInfo {
                        kvdatas: rows,
                        key_type: desc.key,
                        value_type: desc.value,
                    }
                    .print_data();
                    Ok(false)
                }
            }
        }

        Commands::Create { tablename } => {
            let _ = status.dbm.create_table(tablename);
            write_io_success("create table success".to_string())?;
            Ok(false)
        }
        Commands::Delete { tablename } => {
            let _ = status.dbm.delete_table(tablename);
            write_io_success("delete table success".to_string())?;
            Ok(false)
        }
        Commands::New { databasename } => {
            let _ = Database::create(databasename);
            write_io_success("create database success".to_string())?;
            Ok(false)
        }
        Commands::Exit => {
            write_io_success("Exiting ... \n".to_string())?;
            Ok(true)
        }
    }
}
