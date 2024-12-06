use redb::ReadableTable;
use redb::TableDefinition;
use redb::TableHandle;
use redb::{Database, Error};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Default)]
pub struct CommonDbManager {
    pub tablename: String,
    dbpath: String,
}

pub trait CommonDbInterface {
    fn common_get_by_key(&self, key: String) -> Result<String, Error>;
    fn common_insert_by_key(&self, key: String, data: String) -> Result<(), Error>;
    fn common_get_all(&self) -> Result<HashMap<String, String>, Error>;
    fn common_remove_by_key(&self, key: String) -> Result<(), Error>;
    fn common_update_by_key(&self, key: String, data: String) -> Result<(), Error>;
}
impl CommonDbManager {
    pub fn getdb(&self) -> Result<Database, Error> {
        let db_file = Path::new(&self.dbpath);
        if db_file.exists() {
            let db = Database::open(&self.dbpath)?;
            return Ok(db);
        }
        Err(redb::Error::Corrupted("Database not found".to_string()))
    }
    pub fn settablename(&mut self, name: String) -> Result<(), Error> {
        let db = self.getdb()?;
        self.tablename = name.clone();
        let tab_name = self.tablename.clone();
        let tabledefinition: TableDefinition<&str, &str> = TableDefinition::new(tab_name.as_str());
        let read_txn = db.begin_read()?;
        let _ = read_txn.open_table(tabledefinition)?;
        Ok(())
    }
    pub fn setdbpath(&mut self, path: String) -> Result<(), Error> {
        self.dbpath = path;
        self.getdb()?;
        Ok(())
    }

    pub fn gettables(&self) -> Result<Vec<String>, Error> {
        let mut result = Vec::new();
        let db = self.getdb()?;
        let read_txn = db.begin_read()?;
        let x = read_txn.list_tables().unwrap();
        for item in x {
            result.push(item.name().to_string());
        }
        Ok(result)
    }
}

impl CommonDbInterface for CommonDbManager {
    fn common_get_by_key(&self, key: String) -> Result<String, Error> {
        let db = self.getdb()?;
        let tab_name = self.tablename.clone();
        let tabledefinition: TableDefinition<&str, &str> = TableDefinition::new(tab_name.as_str());
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(tabledefinition)?;

        let binding = table.get(&key.as_str())?;
        if let Some(binding) = binding {
            let name_str = binding.value();
            Ok(name_str.to_string())
        } else {
            Err(Error::Corrupted("Key not found".to_string()))
        }
    }
    fn common_insert_by_key(&self, key: String, data: String) -> Result<(), Error> {
        let db = self.getdb()?;
        let tab_name = self.tablename.clone();
        let tabledefinition: TableDefinition<&str, &str> = TableDefinition::new(tab_name.as_str());
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(tabledefinition)?;
            table.insert(&key.as_str(), &data.as_str())?;
        }
        write_txn.commit()?;
        Ok(())
    }
    fn common_get_all(&self) -> Result<HashMap<String, String>, Error> {
        let db = self.getdb()?;
        let tab_name = self.tablename.clone();
        let tabledefinition: TableDefinition<&str, &str> = TableDefinition::new(tab_name.as_str());
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(tabledefinition)?;
        // println!("start get all data....");
        let mut result = HashMap::new();
        let mut iter = table.range::<&str>(..)?;
        while let Some((k, v)) = iter.next().transpose()? {
            result.insert(k.value().to_string(), v.value().to_string());
        }
        Ok(result)
    }
    fn common_remove_by_key(&self, key: String) -> Result<(), Error> {
        let db = self.getdb()?;
        let tab_name = self.tablename.clone();
        let tabledefinition: TableDefinition<&str, &str> = TableDefinition::new(tab_name.as_str());
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(tabledefinition)?;
            table.remove(&key.as_str())?;
        }
        write_txn.commit()?;
        Err(redb::Error::Corrupted("Database not found".to_string()))
    }
    fn common_update_by_key(&self, key: String, data: String) -> Result<(), Error> {
        let db = self.getdb()?;
        let tab_name = self.tablename.clone();
        let tabledefinition: TableDefinition<&str, &str> = TableDefinition::new(tab_name.as_str());
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(tabledefinition)?;
            table.insert(&key.as_str(), &data.as_str())?;
        }
        write_txn.commit()?;
        Err(redb::Error::Corrupted("Database not found".to_string()))
    }
}
