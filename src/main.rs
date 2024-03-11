use clap::{Parser, Subcommand, Args};
use redbcli::redbcontrol::{CommonDbInterface, CommonDbManager};
use std::io::Write;

#[derive(Default)]
pub struct CliStatus {
    tablename: String,
    filepath: String,
    dbm: CommonDbManager,
}

fn main() -> Result<(), String> {
    let mut clistatus = CliStatus::default();
    loop {
        let line = readline()?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match respond(line, &mut clistatus) {
            Ok(quit) => {
                if quit {
                    break;
                }
            }
            Err(err) => {
                write!(std::io::stdout(), "{err}").map_err(|e| e.to_string())?;
                std::io::stdout().flush().map_err(|e| e.to_string())?;
            }
        }
    }

    Ok(())
}

fn write_io(data: String) -> Result<(), String> {
    write!(std::io::stdout(), "{}", format!("-> {} \n", data)).map_err(|e| e.to_string())?;
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    Ok(())
}

fn respond(line: &str, status: &mut CliStatus) -> Result<bool, String> {
    let args = shlex::split(line).ok_or("error: Invalid quoting")?;
    let cli = Cli::try_parse_from(args).map_err(|e| e.to_string())?;
    match cli.command {
        Commands::Set { filepath } => {
            status
                .dbm
                .setdbpath(filepath.clone())
                .map_err(|e| e.to_string())?;
            write_io(format!("set database success!"))?;
            status.filepath = filepath;
        }

        Commands::Use { tablename } => {
            if status.filepath.is_empty() {
                write_io(format!("you must set file path first !!"))?;
                return Ok(false);
            }
            status.dbm.settablename(tablename.clone()).map_err(|e|e.to_string())?;
            status.tablename = tablename.clone();
            write_io(format!("Use table {}", tablename))?;
        }
        Commands::Info (subcmd) => {
            let sub_cmd = subcmd.command.unwrap_or(InfoCommands::Tables);
            match sub_cmd{
                InfoCommands::Tables => {
                    let result = status.dbm.gettables().map_err(|e| e.to_string())?;
                    write_io(format!("data \n{:?}", result))?;
                    return Ok(false);
                },
                InfoCommands::Key { key } =>{
                    if status.tablename.is_empty() {
                            write_io(format!("you must use table to select !!"))?;
                            return Ok(false);
                        }
                         let result = status
                        .dbm
                        .common_get_by_key(key)
                        .map_err(|e| e.to_string())?;
                    write_io(format!("data \n{}", result))?;
                },
                InfoCommands::Table { tablename } => {
                    status.tablename = tablename.clone();
                    status.dbm.settablename(tablename.clone()).map_err(|e| e.to_string())?;
                    let result = status.dbm.common_get_all().map_err(|e| e.to_string())?;
                        write_io(format!("data \n{}", result))?;
                        return Ok(false);
                },
            }
        }
        Commands::Exit => {
            write_io("Exiting ... \n".to_string())?;
            return Ok(true);
        }
    }
    Ok(false)
}

#[derive(Debug, Parser)]
#[command(multicall = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(arg_required_else_help = true)]
    #[command(about = "Set db filepath  ex: set '/home/test.redb'", long_about = None)]
    Set {
        ///set file path
        filepath: String,
    },
    #[command(arg_required_else_help = false)]
    #[command(about = "use db table name    ex : 'use $tablename'", long_about = None)]
    Use {
        tablename:String,
    },

    #[command(arg_required_else_help = false)]
    #[command(about = "info db data ex :'help show'", long_about = None)]
    Info(InfoArgs),
    Exit,
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
#[command(flatten_help = true)]
struct InfoArgs{
    #[command(subcommand)]
    command: Option<InfoCommands>,
}

#[derive(Debug, Subcommand)]
enum InfoCommands{
    //show all tables
    #[command(about = "show all table name ", long_about = None)]
    Tables,
    //show key
    #[command(short_flag='k',about = "use key get data", long_about = None)]
    Key{
        key:String,
    },
    //show table data
    #[command(short_flag= 't',about = "get table data", long_about = None)]
    Table{
        tablename:String
    }

}
fn readline() -> Result<String, String> {
    write!(std::io::stdout(), "$ ").map_err(|e| e.to_string())?;
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    let mut buffer = String::new();
    std::io::stdin()
        .read_line(&mut buffer)
        .map_err(|e| e.to_string())?;
    Ok(buffer)
}
