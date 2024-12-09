use std::collections::HashMap;
use std::io::Write;
pub mod flags;
pub mod pretty_print;
pub mod redbcontrol;

macro_rules! write_io {
    ($func_name:ident, $prefix:literal) => {
        pub fn $func_name(data: String) -> Result<(), String> {
            let out_data = format!("{}-> {} \n", $prefix, data);
            write!(std::io::stdout(), "{}", out_data).map_err(|e| e.to_string())?;
            std::io::stdout().flush().map_err(|e| e.to_string())?;
            Ok(())
        }
    };
}

write_io!(write_io_error, "error:");
write_io!(write_io_success, "success:");
write_io!(write_io_info, "info:");

pub struct TableInfo {
    pub tablename: Vec<String>,
}

pub struct KvInfo {
    pub kvdatas: HashMap<String, String>,
}
