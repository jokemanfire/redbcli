use redb::{
    Database, Error, ReadableTable, TableDefinition,
    TableHandle,
};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Default)]
pub struct CommonDbManager {
    pub tablename: String,
    pub dbpath: String,
}
// Deal with table
pub trait DealTable {
    fn create_table(&self, key: String) -> Result<(), Error>;
    fn delete_table(&self, key: String) -> Result<(), Error>;
    fn list_table(&self) -> Result<Vec<String>, Error>;
}

// Deal with data
pub trait DealData {
    fn get_by_key(&self, key: String) -> Result<String, Error>;
    fn insert_by_key(&self, key: String, data: String) -> Result<(), Error>;
    fn get_all(&self) -> Result<HashMap<String, String>, Error>;
    fn remove_by_key(&self, key: String) -> Result<(), Error>;
    fn update_by_key(&self, key: String, data: String) -> Result<(), Error>;
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
}
impl DealTable for CommonDbManager {
    fn create_table(&self, key: String) -> Result<(), Error> {
        let db = self.getdb()?;
        let read_txn = db.begin_read()?;
        let tabledefinition: TableDefinition<&str, &str> = TableDefinition::new(key.as_str());
        match read_txn.open_table(tabledefinition) {
            Ok(_) => Ok(()),
            Err(e) => Err(Error::Corrupted(e.to_string())),
        }
    }

    fn delete_table(&self, key: String) -> Result<(), Error> {
        let db = self.getdb()?;
        let write_txn = db.begin_write()?;
        {
            let tabledefinition: TableDefinition<&str, &str> = TableDefinition::new(key.as_str());
            match write_txn.delete_table(tabledefinition) {
                Ok(_) => Ok(()),
                Err(e) => Err(Error::Corrupted(e.to_string())),
            }
        }
    }

    fn list_table(&self) -> Result<Vec<String>, Error> {
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
    fn get_by_key(&self, key: String) -> Result<String, Error> {
        let db: Database = self.getdb()?;
        let tabledefinition: TableDefinition<&str, &str> =
            TableDefinition::new(self.tablename.as_str());
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

    fn insert_by_key(&self, key: String, data: String) -> Result<(), Error> {
        let db = self.getdb()?;
        let tabledefinition: TableDefinition<&str, &str> =
            TableDefinition::new(self.tablename.as_str());
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(tabledefinition)?;
            table.insert(&key.as_str(), &data.as_str())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    fn get_all(&self) -> Result<HashMap<String, String>, Error> {
        let db = self.getdb()?;
        let tabledefinition: TableDefinition<&str, &str> =
            TableDefinition::new(self.tablename.as_str());
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(tabledefinition)?;
        let mut result = HashMap::new();
        while let Some((k, v)) = table.iter()?.next().transpose()? {
            result.insert(k.value().to_string(), v.value().to_string());
        }
        Ok(result)
    }
    fn remove_by_key(&self, key: String) -> Result<(), Error> {
        let db = self.getdb()?;
        let tabledefinition: TableDefinition<&str, &str> =
            TableDefinition::new(self.tablename.as_str());
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(tabledefinition)?;
            table.remove(&key.as_str())?;
        }
        write_txn.commit()?;
        Err(redb::Error::Corrupted("Database not found".to_string()))
    }
    fn update_by_key(&self, key: String, data: String) -> Result<(), Error> {
        let db = self.getdb()?;
        let tabledefinition: TableDefinition<&str, &str> =
            TableDefinition::new(self.tablename.as_str());
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(tabledefinition)?;
            table.insert(&key.as_str(), &data.as_str())?;
        }
        write_txn.commit()?;
        Err(redb::Error::Corrupted("Database not found".to_string()))
    }
}
