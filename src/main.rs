use clap::Parser;
use redb::Database;
use redbcli::flags::{Binflags, Cli};
use redbcli::pretty_print::PrintTable;
use redbcli::redbcontrol::DealTable;
use redbcli::{
    flags::{Commands, InfoCommands},
    redbcontrol::{CommonDbManager, DealData},
};
use redbcli::{KvInfo, TableInfo};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::collections::HashMap;
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
                        write_io(err)?;
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

fn write_io(data: String) -> Result<(), String> {
    let out_data = format!("-> {} \n", data);
    write!(std::io::stdout(), "{}", out_data).map_err(|e| e.to_string())?;
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    Ok(())
}

fn respond(line: &str, status: &mut CliStatus) -> Result<bool, String> {
    let args = shlex::split(line).ok_or("error: Invalid quoting")?;
    let cli = Cli::try_parse_from(args).map_err(|e| e.to_string())?;
    if !status.filepath.is_empty() {
        status
            .dbm
            .setdbpath(status.filepath.clone())
            .map_err(|e| e.to_string())?;
    }
    if !status.tablename.is_empty() {
        status
            .dbm
            .settablename(status.tablename.clone())
            .map_err(|e| e.to_string())?;
    }
    match cli.command {
        Commands::Set { filepath } => {
            write_io("set database success!".to_string())?;
            status.filepath = filepath;
            return Ok(false);
        }

        Commands::Use { tablename } => {
            if status.filepath.is_empty() {
                return Err("you must set file path first !!".to_string());
            }
            status.tablename = tablename.clone();
            write_io(format!("Use table {}", tablename))?;
            return Ok(false);
        }
        Commands::Edit => {
            if status.tablename.is_empty() {
                return Err("you must set table first !!".to_string());
            }
            let mut temp_file = tempfile::NamedTempFile::new().map_err(|e| e.to_string())?;
            let result = status.dbm.get_all().map_err(|e| e.to_string())?;
            let json_data = serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?;

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
            match serde_json::from_str::<HashMap<String, String>>(&modified_data) {
                Ok(r_data) => {
                    if modified_data == json_data {
                        println!("No changed!");
                        return Ok(false);
                    }
                    result.iter().for_each(|(key, _)| {
                        let _ = status.dbm.remove_by_key(key.to_string());
                    });

                    println!("Save data to update the database");
                    r_data.iter().for_each(|(key, value)| {
                        let _ = status.dbm.update_by_key(key.to_string(), value.to_string());
                    });
                    return Ok(false);
                }
                Err(_) => {
                    return Err("This is not a valid json str".to_string());
                }
            };
        }

        Commands::Info(subcmd) => {
            let sub_cmd = subcmd.command.unwrap_or(InfoCommands::Tables);
            match sub_cmd {
                InfoCommands::Tables => {
                    if status.tablename.is_empty() {
                        let result = status.dbm.list_table().map_err(|e| e.to_string())?;
                        TableInfo { tablename: result }.print_data();
                        return Ok(false);
                    } else {
                        let result = status.dbm.get_all().map_err(|e| e.to_string())?;
                        KvInfo { kvdatas: result }.print_data();
                        return Ok(false);
                    }
                }
                InfoCommands::Key { key } => {
                    if status.tablename.is_empty() {
                        return Err("you must use table to select !!".to_string());
                    }
                    let result = status.dbm.get_by_key(key.clone()).map_err(|e| e.to_string())?;
                    let mut kvdatas = HashMap::new();
                    kvdatas.insert(key, result);
                    KvInfo {kvdatas}.print_data();
                    return Ok(false);
                }
                InfoCommands::Table { tablename } => {
                    status.tablename = tablename.clone();
                    let result = status.dbm.get_all().map_err(|e| e.to_string())?;
                    KvInfo { kvdatas: result }.print_data();
                    return Ok(false);
                }
            }
        }

        Commands::Create { tablename } => {
            let _ = status.dbm.create_table(tablename);
            write_io("create table success".to_string())?;
            return Ok(false);
        }
        Commands::Delete { tablename } => {
            let _ = status.dbm.delete_table(tablename);
            write_io("delete table success".to_string())?;
            return Ok(false);
        }
        Commands::New { databasename } => {
            let _ = Database::create(databasename);
            write_io("create database success".to_string())?;
            return Ok(false);
        }
        Commands::Exit => {
            write_io("Exiting ... \n".to_string())?;
            return Ok(true);
        }
    }
}
