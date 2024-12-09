use prettytable::{row, Table};

use crate::{KvInfo, TableInfo};

pub trait PrintTable {
    fn print_data(&self);
}

impl PrintTable for TableInfo {
    fn print_data(&self) {
        let mut table = Table::new();
        table.add_row(row![Fgb=>"id", "name"]);
        let mut cnt = 1;
        self.tablename.iter().for_each(|t| {
            table.add_row(row![cnt.to_string(), &t]);
            cnt += 1
        });
        table.printstd();
    }
}

impl PrintTable for KvInfo {
    fn print_data(&self) {
        let mut table = Table::new();
        table.add_row(row![Fgb=>"id", "key", "value"]);
        let mut cnt = 1;
        self.kvdatas.iter().for_each(|(k, v)| {
            table.add_row(row![cnt.to_string(), k, v]);
            cnt += 1
        });
        table.printstd();
    }
}
