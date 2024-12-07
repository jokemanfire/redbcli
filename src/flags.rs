use clap::{self, Args, Parser, Subcommand};
#[derive(clap::Parser)]
#[command(name = "readbcli")]
#[command(about = "readbcli for control redb", long_about = None)]
pub struct Binflags {
    #[arg(long, help = "redb database path", default_value = None)]
    pub path: Option<String>,
}

#[derive(Debug, Parser)]
#[command(multicall = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(arg_required_else_help = true)]
    #[command(about = "Set db filepath  ex: set '/home/test.redb'", long_about = None)]
    Set {
        ///set file path
        filepath: String,
    },
    #[command(arg_required_else_help = false)]
    #[command(about = "use db table name    ex : 'use $tablename'", long_about = None)]
    Use {
        tablename: String,
    },

    #[command(arg_required_else_help = false)]
    #[command(about = "info db data ex :'help show'", long_about = None)]
    Info(InfoArgs),

    #[command(short_flag='e',about = "edit table data", long_about = None)]
    Edit,

    //todo create table
    #[command(short_flag='c',about = "Create a table", long_about = None)]
    Create {
        tablename: String,
    },
    #[command(short_flag='d',about = "Delete a table", long_about = None)]
    Delete {
        tablename: String,
    },

    #[command(short_flag='n',about = "Create a database", long_about = None)]
    New {
        databasename: String,
    },
    Exit,
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
#[command(flatten_help = true)]
pub struct InfoArgs {
    #[command(subcommand)]
    pub command: Option<InfoCommands>,
}

#[derive(Debug, Subcommand)]
pub enum InfoCommands {
    //show all tables
    #[command(about = "show all table name ", long_about = None)]
    Tables,
    //show key
    #[command(short_flag='k',about = "use key get data", long_about = None)]
    Key { key: String },
    //show table data
    #[command(short_flag='t',about = "get table data", long_about = None)]
    Table { tablename: String },
}
