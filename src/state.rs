use crate::Arc;

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
}

impl State {
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
    pub fn get<'a, K>(&'a self, key: K, cf_name: &'a str) -> Option<String>
    where
        K: AsRef<[u8]>,
    {
        if let Some(cf) = self.db.cf_handle(cf_name) {
            if let Ok(Some(value)) = self.db.get_cf(&cf, key) {
                Some(unsafe { String::from_utf8_unchecked(value) })
            } else {
                None
            }
        } else {
            None
        }
    }
    pub fn put<'a, K, V>(&'a self, key: K, value: V, cf_name: &'a str) -> Result<(), DBFailure>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        if let Some(cf) = self.db.cf_handle(cf_name) {
            match self.db.put_cf(&cf, key, value) {
                Ok(_) => Ok(()),
                Err(error) => Err(DBFailure::Error(error)),
            }
        } else {
            Err(DBFailure::CfError)
        }
    }
}
