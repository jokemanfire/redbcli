use comfy_table::{Cell, Table};

use crate::{KvInfo, TableInfo};

pub trait PrintTable {
    fn print_data(&self);
}

impl PrintTable for TableInfo {
    fn print_data(&self) {
        let mut table = Table::new();
        table
            .load_preset(comfy_table::presets::UTF8_FULL)
            .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
            .set_content_arrangement(comfy_table::ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("id"),
                Cell::new("name"),
                Cell::new("key type"),
                Cell::new("value type"),
            ]);
        self.tables.iter().enumerate().for_each(|(idx, t)| {
            table.add_row(vec![
                Cell::new((idx + 1).to_string()),
                Cell::new(&t.name),
                Cell::new(&t.key_type),
                Cell::new(&t.value_type),
            ]);
        });
        println!("{table}");
    }
}

impl PrintTable for KvInfo {
    fn print_data(&self) {
        println!(
            "table type: key = {}, value = {}, rows = {}",
            self.key_type,
            self.value_type,
            self.kvdatas.len()
        );
        let mut table = Table::new();
        table
            .load_preset(comfy_table::presets::UTF8_FULL)
            .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
            .set_content_arrangement(comfy_table::ContentArrangement::Dynamic)
            .set_header(vec![Cell::new("id"), Cell::new("key"), Cell::new("value")]);
        self.kvdatas.iter().enumerate().for_each(|(idx, (k, v))| {
            table.add_row(vec![
                Cell::new((idx + 1).to_string()),
                Cell::new(k),
                Cell::new(v),
            ]);
        });
        println!("{table}");
    }
}
