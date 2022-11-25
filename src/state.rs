use crate::Arc;
use chrono::{self, Utc};
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Clone)]
pub struct State {
    pub db: Arc<crate::rocksdb::DB>,
    pub cache: rocksdb::Cache,
}

// const HTML_HELLO: Html<&str> = Html(HTML);

#[derive(Debug)]
pub enum DBFailure {
    Error(rocksdb::Error),
    CfError,
    SerError,
}

#[derive(Archive, Deserialize, Serialize, Debug)]
pub struct Entry {
    pub views: u32,
    pub scrapes: u32,
    pub contents: Vec<u8>,
    pub creationdate: i64,
    pub obfuscate: bool,
}

impl Entry {
    pub fn new<V>(contents: V, views: u32, scrapes: u32, obfuscate: bool) -> Self
    where
        Vec<u8>: std::convert::From<V>,
    {
        Entry {
            views,
            scrapes,
            contents: contents.into(),
            creationdate: Utc::now().timestamp(),
            obfuscate,
        }
    }
}

impl State {
    pub fn key_exists<'a, K>(&'a self, key: K, cf_name: &'a str) -> Result<bool, DBFailure>
    where
        K: AsRef<[u8]>,
    {
        let Some(cf) = self.db.cf_handle(cf_name) else {
            return Err(DBFailure::CfError)};
        match self.db.get_pinned_cf(&cf, key) {
            Err(error) => Err(DBFailure::Error(error)),
            Ok(None) => Ok(false),
            Ok(Some(_)) => Ok(true),
        }
    }
    pub fn delete<'a, K>(&'a self, key: K, cf_name: &'a str) -> Result<(), DBFailure>
    where
        K: AsRef<[u8]>,
    {
        let Some(cf) = self.db.cf_handle(cf_name) else {
            return Err(DBFailure::CfError)};
        match self.db.delete_cf(&cf, key) {
            Err(error) => Err(DBFailure::Error(error)),
            Ok(_) => Ok(()),
        }
    }
    pub fn get_bytes<'a, K>(&'a self, key: K, cf_name: &'a str) -> Option<Vec<u8>>
    where
        K: AsRef<[u8]>,
    {
        let Some(cf) = self.db.cf_handle(cf_name) else {
            return None};
        let Ok(Some(value)) = self.db.get_cf(&cf, key) else {
            return None};
        Some(value)
    }
    pub fn get<'a, K>(&'a self, key: K, cf_name: &'a str) -> Option<Entry>
    where
        K: AsRef<[u8]>,
    {
        let Some(cf) = self.db.cf_handle(cf_name)  else{
            return None
        };
        let Ok(Some(value)) = self.db.get_cf(&cf, key) else {
            return None
        };
        let Ok(entry) = (unsafe { rkyv::from_bytes_unchecked::<Entry>(&value) }) else {
            return None
        };
        Some(entry)
    }
    pub fn put_large<'a, K>(
        &'a self,
        key: K,
        value: Entry,
        cf_name: &'a str,
    ) -> Result<(), DBFailure>
    where
        K: AsRef<[u8]>,
    {
        let Some(cf) = self.db.cf_handle(cf_name) else {
            return Err(DBFailure::CfError)
        };
        let Ok(value) = rkyv::to_bytes::<_, 4096>(&value) else {
            return Err(DBFailure::SerError)
        };
        match self.db.put_cf(&cf, key, value) {
            Err(error) => Err(DBFailure::Error(error)),
            Ok(_) => Ok(()),
        }
    }
    pub fn put<'a, K>(&'a self, key: K, value: Entry, cf_name: &'a str) -> Result<(), DBFailure>
    where
        K: AsRef<[u8]>,
    {
        let Some(cf) = self.db.cf_handle(cf_name) else {
            return Err(DBFailure::CfError)};
        let Ok(value) = rkyv::to_bytes::<_, 256>(&value) else {
            return Err(DBFailure::SerError)};
        match self.db.put_cf(&cf, key, value) {
            Err(error) => Err(DBFailure::Error(error)),
            Ok(_) => Ok(()),
        }
    }
}
