use std::io::Write;
use redbcli::redbcontrol::{CommonDbManager, CommonDbInterface};
use clap::{Parser, Subcommand};


#[derive(Default)]
pub struct CliStatus{
    tablename:String,
    filepath:String,
    dbm:CommonDbManager,
}



fn main() -> Result<(), String> {
    let mut clistatus = CliStatus::default();
    loop {
       
        let line = readline()?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match respond(line,&mut clistatus) {
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

fn write_io(data:String)->Result<(),String>{
    write!(std::io::stdout(), "{}", format!("-> {} \n",data)).map_err(|e| e.to_string())?;
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    Ok(())
}

fn respond(line: &str,status:&mut CliStatus) -> Result<bool, String> {
    let args = shlex::split(line).ok_or("error: Invalid quoting")?;
    let cli = Cli::try_parse_from(args).map_err(|e| e.to_string())?;
    match cli.command {
        Commands::Set { filepath }=>{
            status.dbm.setdbpath(filepath.clone()).map_err(|e|e.to_string())?;
            write_io(format!("set database success!"))?;
            status.filepath = filepath;
        }

        Commands::Use{tablename} => {
            if status.filepath.is_empty(){
                write_io(format!("you must set file path first !!"))?;
                return Ok(false);
            }
            status.tablename = tablename.clone();
            status.dbm.settablename(tablename.clone());
            write_io(format!("Use table {}",tablename))?;
        }
        Commands::Info {key}=> {
            if status.tablename.is_empty(){
                write_io(format!("you must use table to select !!"))?;
                return Ok(false);
            }
            if key == "*".to_string(){
                let result = status.dbm.common_get_all().map_err(|e| e.to_string())?;
                write_io(format!("data \n{}",result))?;
                return Ok(false);
            }
            let result = status.dbm.common_get_by_key(key).map_err(|e| e.to_string())?;
            write_io(format!("data \n{}",result))?;
           
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
    Set{
        filepath:String
    },

    #[command(arg_required_else_help = true)]
    Use{
        /// choose table
        tablename:String
    },
    #[command(arg_required_else_help = false)]
    Info {
        /// key
        key:String
    },
    Exit,
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