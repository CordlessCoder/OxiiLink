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
    // pub fn incr<'a, K>(&'a self, key: K, cf_name: &'a str) -> Result<(), DBFailure>
    // where
    //     K: AsRef<[u8]>,
    // {
    //     self.db.mege
    // }
    pub fn key_exists<'a, K>(&'a self, key: K, cf_name: &'a str) -> Result<bool, DBFailure>
    where
        K: AsRef<[u8]>,
    {
        if let Some(cf) = self.db.cf_handle(cf_name) {
            match self.db.get_pinned_cf(&cf, key) {
                Ok(Some(_)) => Ok(true),
                Ok(None) => Ok(false),
                Err(error) => Err(DBFailure::Error(error)),
            }
        } else {
            Err(DBFailure::CfError)
        }
    }
    pub fn delete<'a, K>(&'a self, key: K, cf_name: &'a str) -> Result<(), DBFailure>
    where
        K: AsRef<[u8]>,
    {
        if let Some(cf) = self.db.cf_handle(cf_name) {
            match self.db.delete_cf(&cf, key) {
                Ok(_) => Ok(()),
                Err(error) => Err(DBFailure::Error(error)),
            }
        } else {
            Err(DBFailure::CfError)
        }
    }
    pub fn get_bytes<'a, K>(&'a self, key: K, cf_name: &'a str) -> Option<Vec<u8>>
    where
        K: AsRef<[u8]>,
    {
        if let Some(cf) = self.db.cf_handle(cf_name) {
            if let Ok(Some(value)) = self.db.get_cf(&cf, key) {
                Some(value)
            } else {
                None
            }
        } else {
            None
        }
    }
    pub fn get<'a, K>(&'a self, key: K, cf_name: &'a str) -> Option<Entry>
    where
        K: AsRef<[u8]>,
    {
        if let Some(cf) = self.db.cf_handle(cf_name) {
            if let Ok(Some(value)) = self.db.get_cf(&cf, key) {
                if let Ok(entry) = unsafe { rkyv::from_bytes_unchecked::<Entry>(&value) } {
                    Some(entry)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
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
        if let Some(cf) = self.db.cf_handle(cf_name) {
            if let Ok(value) = rkyv::to_bytes::<_, 4096>(&value) {
                match self.db.put_cf(&cf, key, value) {
                    Ok(_) => Ok(()),
                    Err(error) => Err(DBFailure::Error(error)),
                }
            } else {
                Err(DBFailure::SerError)
            }
        } else {
            Err(DBFailure::CfError)
        }
    }
    pub fn put<'a, K>(&'a self, key: K, value: Entry, cf_name: &'a str) -> Result<(), DBFailure>
    where
        K: AsRef<[u8]>,
    {
        if let Some(cf) = self.db.cf_handle(cf_name) {
            if let Ok(value) = rkyv::to_bytes::<_, 256>(&value) {
                match self.db.put_cf(&cf, key, value) {
                    Ok(_) => Ok(()),
                    Err(error) => Err(DBFailure::Error(error)),
                }
            } else {
                Err(DBFailure::SerError)
            }
        } else {
            Err(DBFailure::CfError)
        }
    }
}
