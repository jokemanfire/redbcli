use std::collections::HashMap;

pub mod flags;
pub mod pretty_print;
pub mod redbcontrol;

pub struct TableInfo {
    pub tablename: Vec<String>,
}

pub struct KvInfo {
    pub kvdatas: HashMap<String, String>,
}
