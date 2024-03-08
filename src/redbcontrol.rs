
use std::fs::File;
use std::io::SeekFrom;
use std::io::prelude::*;
use std::path::Path;
use redb::ReadableTable;
use redb::{Database,Error};
use redb::TableDefinition;

fn read_header(path:String) -> Result<(), Error> {
    let mut file = File::open(path)?;
    let mut buffer = [0u8; 64];
    file.read(&mut buffer)?;
    println!("{:?}", buffer);
    Ok(())
}

fn read_seeker(path:String,start_i:u64,lens:u64) -> Result<(), Error> {
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(start_i))?;
    let mut buffer = vec![0u8; lens.try_into().unwrap() ];
    file.read(&mut buffer)?;
    println!("{:?}", buffer);
    Ok(())
}
#[derive(Debug, Default)]
pub struct CommonDbManager {
    pub(crate) tablename: String,
    dbpath:String,
}

/// T :represent Struct
/// F :represent Filter

pub trait CommonDbInterface {
    fn common_get_by_key(&self, key: String) -> Result<String, Error>;
    fn common_insert_by_key(&self, key: String, data: String) -> Result<(), Error>;
    fn common_get_all(&self) -> Result<String, Error>;
    fn common_remove_by_key(&self, key: String) -> Result<(), Error>;
    fn common_update_by_key(&self, key: String, data:String) -> Result<(), Error>;
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
    pub fn settablename(&mut self,name:String){
        self.tablename = name;
    }
    pub fn setdbpath(&mut self,path:String)-> Result<(),Error>{
        self.dbpath = path;
        self.getdb()?;
        Ok(())
    }
    
}

impl CommonDbInterface for CommonDbManager
{
    fn common_get_by_key(&self, key: String) -> Result<String, Error> {
        let db = self.getdb()?;
        let tab_name = self.tablename.clone();
        let tabledefinition: TableDefinition<&str, &str> = TableDefinition::new(tab_name.as_str());
        //if not exits
        let write_txn = db.begin_write()?;
        {
            write_txn.open_table(tabledefinition)?;
        }
        write_txn.commit()?;
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(tabledefinition)?;
        /* Started by AICoder, pid:4f5b7ee71e4f4dc1b5f78075134b8c76 */
        let binding = table.get(&key.as_str())?;
        if let Some(binding) = binding {
            let name_str = binding.value();
            Ok(name_str.to_string())
        } else {
            Err(Error::Corrupted("Key not found".to_string()))
        }
        /* Ended by AICoder, pid:4f5b7ee71e4f4dc1b5f78075134b8c76 */
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
    fn common_get_all(&self) -> Result<String, Error> {
        //if not exits
        let db = self.getdb()?;
        let tab_name = self.tablename.clone();
        let tabledefinition: TableDefinition<&str, &str> = TableDefinition::new(tab_name.as_str());
        //if not exits
        let write_txn = db.begin_write()?;
        {
            write_txn.open_table(tabledefinition)?;
        }
        write_txn.commit()?;
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(tabledefinition)?;
        // println!("start get all data....");
        let mut result = String::new();
        let mut iter = table.range::<&str>(..)?;
        while let Some((k, v)) = iter.next().transpose()? {
            // let formatted_data = serde_json::to_string_pretty(&v.value()).unwrap();
            let r =  format!("Key: {} \n Value: {}\n", k.value(), v.value());
            result += r.as_str();
               
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







#[test]
fn test_read_header()
{
    read_header("src/test.redb".to_string());
}


#[test]
fn test_read_seeker()
{
    read_seeker("src/test.redb".to_string(),64,128);
    read_seeker("src/test.redb".to_string(),64+128,128);
    read_seeker("src/test.redb".to_string(),4096*2,128);
    
}
