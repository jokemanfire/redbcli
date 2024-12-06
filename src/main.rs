use clap::Parser;
use redbcli::flags::{Binflags, Cli};
use redbcli::pretty_print::PrintTable;
use redbcli::{
    flags::{Commands, InfoCommands},
    redbcontrol::{CommonDbInterface, CommonDbManager},
};
use redbcli::{KvInfo, TableInfo};
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
    let history_path = PathBuf::from("/tmp/redbcli/history");
    if !history_path.exists() {
        std::fs::create_dir_all(&history_path).expect("create history failed");
    }
    let mut rl = DefaultEditor::new().unwrap();
    if rl.load_history(&history_path).is_err() {
        println!("No previous history.");
    }
    loop {
        let prompt = format!(
            "\nDB:[{}] TAB:[{}] \n>> ",
            clistatus.filepath, clistatus.tablename
        );
        let readline = rl.readline(&prompt);
        let _ = rl.save_history(&history_path);
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
        let _ = status.dbm.setdbpath(status.filepath.clone());
    }
    if !status.tablename.is_empty() {
        let _ = status.dbm.settablename(status.tablename.clone());
    }
    match cli.command {
        Commands::Set { filepath } => {
            write_io("set database success!".to_string())?;
            status.filepath = filepath;
        }

        Commands::Use { tablename } => {
            if status.filepath.is_empty() {
                write_io("you must set file path first !!".to_string())?;
                return Ok(false);
            }
            status.tablename = tablename.clone();
            write_io(format!("Use table {}", tablename))?;
        }
        Commands::Info(subcmd) => {
            let sub_cmd = subcmd.command.unwrap_or(InfoCommands::Tables);
            match sub_cmd {
                InfoCommands::Tables => {
                    if status.tablename.is_empty() {
                        let result = status.dbm.gettables().map_err(|e| e.to_string())?;
                        TableInfo { tablename: result }.print_data();
                        // write_io(format!("data \n{:?}", result))?;
                        return Ok(false);
                    } else {
                        let result = status.dbm.common_get_all().map_err(|e| e.to_string())?;
                        KvInfo { kvdatas: result }.print_data();
                        return Ok(false);
                    }
                }
                InfoCommands::Key { key } => {
                    if status.tablename.is_empty() {
                        write_io("you must use table to select !!".to_string())?;
                        return Ok(false);
                    }
                    let result = status
                        .dbm
                        .common_get_by_key(key)
                        .map_err(|e| e.to_string())?;
                    write_io(format!("data \n{}", result))?;
                }
                InfoCommands::Table { tablename } => {
                    status.tablename = tablename.clone();
                    let result = status.dbm.common_get_all().map_err(|e| e.to_string())?;
                    KvInfo { kvdatas: result }.print_data();
                    return Ok(false);
                }
            }
        }
        Commands::Exit => {
            write_io("Exiting ... \n".to_string())?;
            return Ok(true);
        }
    }
    Ok(false)
}
