use crate::{StatusCode, FILES_DIR, IP};
use axum::http::header::HeaderName;
use axum::http::HeaderMap;
use axum::response::Html;
use axum::{response::IntoResponse, routing::get_service};
use lazy_static::lazy_static;
use std::fs::File;
use std::io::Read;
use tower_http::services::{ServeDir, ServeFile};

pub fn make_descriptors(
    opts: crate::rocksdb::Options,
    cf_names: Vec<&str>,
) -> Vec<crate::rocksdb::ColumnFamilyDescriptor> {
    cf_names
        .into_iter()
        .map(|x| crate::rocksdb::ColumnFamilyDescriptor::new(x, opts.clone()))
        .collect()
}

pub fn serve() -> axum::routing::MethodRouter {
    get_service(ServeDir::new(FILES_DIR)).handle_error(handle_error)
}
//
pub fn serve_file(file: &str) -> axum::routing::MethodRouter {
    get_service(ServeFile::new(file)).handle_error(handle_error)
}

async fn handle_error(_err: std::io::Error) -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong...")
}

pub async fn web_short() -> Html<String> {
    WEB_SHORT.to_owned()
}

pub async fn web_paste() -> Html<String> {
    WEB_PASTE.to_owned()
}

pub async fn root(headers: HeaderMap) -> impl IntoResponse {
    let html = match headers.get(HeaderName::from_static("accept")) {
        Some(a) => a.to_str().unwrap_or("").contains("html"),
        None => false,
    };
    if !html {
        HELLO.to_owned().into_response()
    } else {
        HTML_HELLO.to_owned().into_response()
    }
}

lazy_static! {
    pub static ref HELLO: String = {
        let mut file = File::open(FILES_DIR.to_owned() + "/HELLO").unwrap();
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();
        data.replace(r"{IP_ADDR}", IP)
    };
    pub static ref HTML_HELLO: Html<String> = Html({
        let mut file = File::open(FILES_DIR.to_owned() + "/HELLO.html").unwrap();
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();
        data.replace(r"{IP_ADDR}", IP)
    },);
    pub static ref WEB_SHORT: Html<String> = Html({
        let mut file = File::open(FILES_DIR.to_owned() + "/WEB_SHORT.html").unwrap();
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();
        data.replace(r"{IP_ADDR}", IP)
    },);
    pub static ref WEB_PASTE: Html<String> = Html({
        let mut file = File::open(FILES_DIR.to_owned() + "/WEB_PASTE.html").unwrap();
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();
        data.replace(r"{IP_ADDR}", IP)
    },);
}
