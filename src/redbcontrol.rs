use crate::dynread::{self, TableTypeDesc};
use crate::CliError;
use redb::{Database, Error, ReadableDatabase, TableDefinition, TableError, TableHandle};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Default)]
pub struct CommonDbManager {
    pub tablename: String,
    pub dbpath: String,
}
// Deal with table
pub trait DealTable {
    fn create_table(&self, key: String) -> Result<(), CliError>;
    fn delete_table(&self, key: String) -> Result<(), CliError>;
    fn list_table(&self) -> Result<Vec<String>, CliError>;
}

// Deal with data
pub trait DealData {
    fn get_by_key(&self, key: String) -> Result<String, CliError>;
    fn insert_by_key(&self, key: String, data: String) -> Result<(), CliError>;
    fn get_all(&self) -> Result<HashMap<String, String>, CliError>;
    fn remove_by_key(&self, key: String) -> Result<(), CliError>;
    fn update_by_key(&self, key: String, data: String) -> Result<(), CliError>;
}

fn table_lookup_error(e: TableError, name: &str) -> CliError {
    match e {
        TableError::TableDoesNotExist(_) => {
            CliError::Redb(Error::Corrupted(format!("Table '{name}' does not exist")))
        }
        TableError::TableIsMultimap(_) => CliError::Redb(Error::Corrupted(format!(
            "Table '{name}' is a multimap table, which is not supported yet"
        ))),
        e => e.into(),
    }
}

fn str_op_error(e: TableError, name: &str) -> CliError {
    match e {
        TableError::TableTypeMismatch { key, value, .. } => CliError::Redb(Error::Corrupted(format!(
            "table '{name}' is of type Table<{}, {}>; only &str -> &str tables support this operation, use 'info table {name}' to browse it",
            key.name(),
            value.name()
        ))),
        e => table_lookup_error(e, name),
    }
}

impl CommonDbManager {
    pub fn getdb(&self) -> Result<Database, CliError> {
        let db_file = Path::new(&self.dbpath);
        if db_file.exists() {
            let db = Database::open(&self.dbpath)?;
            return Ok(db);
        }
        Err(redb::Error::Corrupted("Database not found".to_string()).into())
    }
    pub fn settablename(&mut self, name: String) -> Result<(), CliError> {
        let db = self.getdb()?;
        let read_txn = db.begin_read()?;
        let handle: TableDefinition<&str, &str> = TableDefinition::new(name.as_str());
        read_txn
            .open_untyped_table(handle)
            .map_err(|e| table_lookup_error(e, &name))?;
        self.tablename = name;
        Ok(())
    }
    pub fn setdbpath(&mut self, path: String) -> Result<(), CliError> {
        self.dbpath = path;
        self.getdb()?;
        Ok(())
    }
    pub fn table_type(&self, name: &str) -> Result<TableTypeDesc, CliError> {
        let db = self.getdb()?;
        let read_txn = db.begin_read()?;
        dynread::probe_table_type(&read_txn, name).map_err(|e| table_lookup_error(e, name))
    }
    pub fn list_table_types(&self) -> Result<Vec<(String, TableTypeDesc)>, CliError> {
        let names = DealTable::list_table(self)?;
        let mut result = Vec::with_capacity(names.len());
        for name in names {
            let desc = self.table_type(&name)?;
            result.push((name, desc));
        }
        Ok(result)
    }
    pub fn get_all_dyn(
        &self,
        name: &str,
    ) -> Result<(TableTypeDesc, Vec<(String, String)>), CliError> {
        let desc = self.table_type(name)?;
        let db = self.getdb()?;
        let read_txn = db.begin_read()?;
        let rows = dynread::read_table_dyn(&read_txn, name, &desc)?;
        Ok((desc, rows))
    }
    /// Replaces the whole content of table `name` with edited rows (cell
    /// strings as produced by `get_all_dyn`). All rows are validated before
    /// anything is written; on error the table is left unchanged.
    pub fn update_all_dyn(&self, name: &str, rows: Vec<(String, String)>) -> Result<(), CliError> {
        let desc = self.table_type(name)?;
        let db = self.getdb()?;
        let write_txn = db.begin_write()?;
        match dynread::write_table_dyn(&write_txn, name, &desc, rows) {
            Ok(()) => {
                write_txn.commit()?;
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }
}
impl DealTable for CommonDbManager {
    fn create_table(&self, key: String) -> Result<(), CliError> {
        let db = self.getdb()?;
        let tabledefinition: TableDefinition<&str, &str> = TableDefinition::new(key.as_str());
        let write_txn = db.begin_write()?;
        {
            write_txn.open_table(tabledefinition)?;
        }
        write_txn.commit()?;
        Ok(())
    }

    fn delete_table(&self, key: String) -> Result<(), CliError> {
        let db = self.getdb()?;
        let write_txn = db.begin_write()?;
        let tabledefinition: TableDefinition<&str, &str> = TableDefinition::new(key.as_str());
        let result = write_txn.delete_table(tabledefinition);
        write_txn.commit()?;
        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(Error::Corrupted(e.to_string()).into()),
        }
    }

    fn list_table(&self) -> Result<Vec<String>, CliError> {
        let db = self.getdb()?;
        let read_txn = db.begin_read()?;
        let result: Vec<String> = read_txn
            .list_tables()?
            .map(|table| table.name().to_string())
            .collect();

        Ok(result)
    }
}
impl DealData for CommonDbManager {
    fn get_by_key(&self, key: String) -> Result<String, CliError> {
        let db: Database = self.getdb()?;
        let tabledefinition: TableDefinition<&str, &str> =
            TableDefinition::new(self.tablename.as_str());
        let read_txn = db.begin_read()?;
        let table = read_txn
            .open_table(tabledefinition)
            .map_err(|e| str_op_error(e, &self.tablename))?;
        let binding = table.get(&key.as_str())?;
        if let Some(binding) = binding {
            let name_str = binding.value();
            Ok(name_str.to_string())
        } else {
            Err(Error::Corrupted("Key not found".to_string()).into())
        }
    }

    fn insert_by_key(&self, key: String, data: String) -> Result<(), CliError> {
        let db = self.getdb()?;
        let tabledefinition: TableDefinition<&str, &str> =
            TableDefinition::new(self.tablename.as_str());
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn
                .open_table(tabledefinition)
                .map_err(|e| str_op_error(e, &self.tablename))?;
            table.insert(&key.as_str(), &data.as_str())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    fn get_all(&self) -> Result<HashMap<String, String>, CliError> {
        let db = self.getdb()?;
        let tabledefinition: TableDefinition<&str, &str> =
            TableDefinition::new(self.tablename.as_str());
        let read_txn = db.begin_read()?;
        let table = read_txn
            .open_table(tabledefinition)
            .map_err(|e| str_op_error(e, &self.tablename))?;
        let mut result = HashMap::new();
        let mut iter = table.range::<&str>(..)?;
        while let Some((k, v)) = iter.next().transpose()? {
            result.insert(k.value().to_string(), v.value().to_string());
        }
        Ok(result)
    }
    fn remove_by_key(&self, key: String) -> Result<(), CliError> {
        let db = self.getdb()?;
        let tabledefinition: TableDefinition<&str, &str> =
            TableDefinition::new(self.tablename.as_str());
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn
                .open_table(tabledefinition)
                .map_err(|e| str_op_error(e, &self.tablename))?;
            table.remove(&key.as_str())?;
        }
        write_txn.commit()?;
        Ok(())
    }
    fn update_by_key(&self, key: String, data: String) -> Result<(), CliError> {
        let db = self.getdb()?;
        let tabledefinition: TableDefinition<&str, &str> =
            TableDefinition::new(self.tablename.as_str());
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn
                .open_table(tabledefinition)
                .map_err(|e| str_op_error(e, &self.tablename))?;
            table.insert(&key.as_str(), &data.as_str())?;
        }
        write_txn.commit()?;
        Ok(())
    }
}
