use crate::{StatusCode, FILES_DIR, IP};
use axum::http::header::HeaderName;
use axum::http::HeaderMap;
use axum::response::Html;
use axum::{response::IntoResponse, routing::get_service};
use html2text::from_read;
use lazy_static::lazy_static;
use regex::Regex;
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

pub async fn web_short(headers: HeaderMap) -> Html<String> {
    use ClientType::*;
    match ClientType::from(&headers) {
        HTML | NoHtml => WEB_SHORT.to_owned(),
        _ => EMBED_SHORT.to_owned(),
    }
}

pub async fn web_paste(headers: HeaderMap) -> Html<String> {
    use ClientType::*;
    match ClientType::from(&headers) {
        HTML | NoHtml => WEB_PASTE.to_owned(),
        _ => EMBED_PASTE.to_owned(),
    }
}

pub fn new_embed(title: &str, description: &str, url: &str, limit: usize) -> Html<String> {
    let length = description.len();
    let description = description.get(0..limit.min(length)).unwrap_or("");
    Html(format!(
        "
<html>
  <head>
    <meta charset='utf-8' />
    <title>{0}</title>
    'meta name='author' content='CordlessCoder' />
    <meta
      name='description'
      content='{1}{2}'
    />
    <meta content='{0}' property='og:title' />
    <meta
      content='{1}{2}'
      property='og:description'
    />
    <meta content='{url}' property='og:url' />
    <meta content='#F7768E' data-react-helmet='true' name='theme-color' />
  </head>
</html>",
        title,
        description,
        if length > limit { "..." } else { "" }
    ))
}

#[derive(Debug, PartialEq)]
pub enum ClientType {
    Discord,
    Slack,
    Twitter,
    WhatsApp,
    NoHtml,
    HTML,
}

impl From<&HeaderMap> for ClientType {
    fn from(headers: &HeaderMap) -> Self {
        use ClientType::*;
        match headers.get(HeaderName::from_static("user-agent")) {
            Some(h_uagent) => {
                if let Ok(uagent) = h_uagent.to_str() {
                    [
                        (Discord, vec!["Discordbot"]),
                        (Twitter, vec!["Twitterbot"]),
                        (WhatsApp, vec!["WhatsApp"]),
                        (Slack, vec!["Slackbot", "Slack-ImgProxy"]),
                    ]
                    .into_iter()
                    .find(|(_, header)| header.into_iter().any(|header| uagent.contains(header)))
                    .unwrap_or((
                        // None of the embed service types matched
                        {
                            match headers.get(HeaderName::from_static("accept")) {
                                Some(a) => {
                                    if a.to_str().unwrap_or("").contains("html") {
                                        HTML
                                    } else {
                                        NoHtml
                                    }
                                }
                                None => NoHtml,
                            }
                        },
                        vec![],
                    ))
                    .0
                } else {
                    NoHtml
                }
            }
            None => NoHtml,
        }
    }
}

pub async fn root(headers: HeaderMap) -> impl IntoResponse {
    use ClientType::*;
    match ClientType::from(&headers) {
        NoHtml => HELLO.to_owned().into_response(),
        HTML => HTML_HELLO.to_owned().into_response(),
        _ => EMBED_HELLO.to_owned().into_response(),
    }
}

fn html_to_text<R>(input: R, width: usize) -> String
where
    R: std::io::Read,
{
    let data = from_read(input, width);
    let re = Regex::new(r"\[(?P<link>[^\[\]]+)\]\[\d{1}\]").unwrap();
    let data = re.replace_all(&data, "$link");
    let re = Regex::new(r"\[(\d*)\]: [ -~]*").unwrap();
    let data = re.replace_all(&data, "");
    let re = Regex::new(r"(?m:^[#]+)").unwrap();
    let data = re.replace_all(&data, "");
    let re = Regex::new(r"(?m:\n \n )").unwrap();
    let data = re.replace_all(&data, "");
    let re = Regex::new(r"(?m:^`(?P<req>[^`]+)*`)").unwrap();
    let data = re.replace_all(&data, "    $req");
    data.replace("CordlessCoder:source", "CordlessCoder") // To remove the :source link from "Made
        // by CordlessCoder:source"
        .trim_end()
        .to_string()
}

lazy_static! {
    pub static ref EMBED_HELLO: Html<String> = Html({
        let mut file = File::open(FILES_DIR.to_owned() + "/EMBED.html").unwrap();
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();
        data.replace(r"{IP_ADDR}", IP)
    },);
    pub static ref EMBED_SHORT: Html<String> = Html({
        let mut file = File::open(FILES_DIR.to_owned() + "/EMBED_SHORT.html").unwrap();
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();
        data.replace(r"{IP_ADDR}", IP)
    },);
    pub static ref EMBED_PASTE: Html<String> = Html({
        let mut file = File::open(FILES_DIR.to_owned() + "/EMBED_PASTE.html").unwrap();
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();
        data.replace(r"{IP_ADDR}", IP)
    },);
    pub static ref HTML_HELLO: Html<String> = Html({
        let mut file = File::open(FILES_DIR.to_owned() + "/HELLO.html").unwrap();
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();
        data.replace(r"{IP_ADDR}", IP)
    },);
    pub static ref HELLO: String = {
        let data = HTML_HELLO.0.to_owned().into_bytes();
        html_to_text(&*data, 65)
    };
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
