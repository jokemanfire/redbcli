use comfy_table::{Cell, Table};

use crate::{KvInfo, TableInfo};

pub trait PrintTable {
    fn print_data(&self);
}

impl PrintTable for TableInfo {
    fn print_data(&self) {
        let mut table = Table::new();
        table.set_header(vec![Cell::new("id"), Cell::new("name")]);
        let mut cnt = 1;
        self.tablename.iter().for_each(|t| {
            table.add_row(vec![Cell::new(cnt.to_string()), Cell::new(t)]);
            cnt += 1
        });
        println!("{table}");
    }
}

impl PrintTable for KvInfo {
    fn print_data(&self) {
        let mut table = Table::new();
        table
            .load_preset(comfy_table::presets::UTF8_FULL)
            .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
            .set_content_arrangement(comfy_table::ContentArrangement::Dynamic)
            .set_header(vec![Cell::new("id"), Cell::new("key"), Cell::new("value")]);
        let mut cnt = 1;
        self.kvdatas.iter().for_each(|(k, v)| {
            table.add_row(vec![Cell::new(cnt.to_string()), Cell::new(k), Cell::new(v)]);
            cnt += 1
        });
        println!("{table}");
    }
}
