use crate::state::State;
use crate::{StatusCode, UrlPath, FILES_DIR, IP, PASTE_CF, URL_CF};
use axum::http::header::HeaderName;
use axum::http::HeaderMap;
use axum::response::Html;
use axum::Extension;
use axum::{response::IntoResponse, routing::get_service};
use chrono::{TimeZone, Utc};
use html2text::from_read;
use lazy_static::lazy_static;
use regex::Regex;
use std::borrow::Cow;
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

pub async fn analytics_paste(
    UrlPath(paste): UrlPath<String>,
    headers: HeaderMap,
    Extension(state): Extension<State>,
) -> Result<impl IntoResponse, StatusCode> {
    let (paste, _) = match paste.split_once('.') {
        Some((paste, ext)) => (paste, Some(ext)),
        None => (paste.as_str(), None),
    };
    let Some(entry) = state.get(paste, PASTE_CF) else {
        return Err(StatusCode::NOT_FOUND)
    };
    use ClientType::*;
    match ClientType::from(&headers) {
        HTML => Ok(Html(format!(
            "<html><head>
<meta name='author' content='CordlessCoder'>
<meta name='description' content='a blazingly-fast URL shortener and pastebin/paste.rs clone
written in Rust using Axum'>
<title>OxiiLink - Pastes done Rusty</title>
<link rel='stylesheet' href='/files/style.css'>
</head><body>
Views: <a>{}</a><br />
Scrapes: <a>{}</a><br />
Created: <a>{}</a>
</body></html>",
            entry.views,
            entry.scrapes,
            Utc.timestamp_opt(entry.creationdate, 0)
                .unwrap()
                .format("%d/%m/%Y %H:%M")
        ))
        .into_response()),
        NoHtml => Ok(format!(
            "Views: {}\nScrapes: {}\nCreated: {}",
            entry.views,
            entry.scrapes,
            Utc.timestamp_opt(entry.creationdate, 0)
                .unwrap()
                .format("%d/%m/%Y %H:%M")
        )
        .into_response()),
        _ => Ok(new_embed(
            &format!("Paste analytics for {paste}"),
            "OxiiLink",
            &format!(
                "Views: {}\nScrapes: {}\nCreated: {}",
                entry.views,
                entry.scrapes,
                Utc.timestamp_opt(entry.creationdate, 0)
                    .unwrap()
                    .format("%d/%m/%Y %H:%M")
            ),
            &format!("{IP}/a/{paste}"),
            120,
        )
        .into_response()),
    }
}
pub async fn analytics_url(
    UrlPath(short): UrlPath<String>,
    headers: HeaderMap,
    Extension(state): Extension<State>,
) -> Result<impl IntoResponse, StatusCode> {
    let Some(entry) = state.get(&short, URL_CF) else {
        return Err(StatusCode::NOT_FOUND)
    };
    use ClientType::*;
    match ClientType::from(&headers) {
        HTML => Ok(Html(format!(
            "<html><head>
<meta name='author' content='CordlessCoder'>
<meta name='description' content='a blazingly-fast URL shortener and pastebin/paste.rs clone
written in Rust using Axum'>
<title>OxiiLink - shortened URL links done Rusty</title>
<link rel='stylesheet' href='/files/style.css'>
</head><body>
Views: <a>{}</a><br />
Scrapes: <a>{}</a><br />
Created: <a>{}</a>
</body></html>",
            entry.views,
            entry.scrapes,
            Utc.timestamp_opt(entry.creationdate, 0)
                .unwrap()
                .format("%d/%m/%Y %H:%M")
        ))
        .into_response()),
        NoHtml => Ok(format!(
            "Views: {}\nScrapes: {}\nCreated: {}",
            entry.views,
            entry.scrapes,
            Utc.timestamp_opt(entry.creationdate, 0)
                .unwrap()
                .format("%d/%m/%Y %H:%M")
        )
        .into_response()),
        _ => Ok(new_embed(
            &format!("Paste analytics for {short}"),
            "OxiiLink",
            &format!(
                "Views: {}\nScrapes: {}\nCreated: {}",
                entry.views,
                entry.scrapes,
                Utc.timestamp_opt(entry.creationdate, 0)
                    .unwrap()
                    .format("%d/%m/%Y %H:%M")
            ),
            &format!("{IP}/a/{short}"),
            120,
        )
        .into_response()),
    }
}

pub async fn web_short(headers: HeaderMap) -> impl IntoResponse {
    use ClientType::*;

    match ClientType::from(&headers) {
        HTML => WEB_SHORT.to_owned().into_response(),
        NoHtml => HELLO.to_owned().into_response(),
        _ => EMBED_SHORT.to_owned().into_response(),
    }
}

pub async fn web_analytics(headers: HeaderMap) -> impl IntoResponse {
    use ClientType::*;

    match ClientType::from(&headers) {
        HTML => WEB_ANALYTICS.to_owned().into_response(),
        NoHtml => HELLO.to_owned().into_response(),
        _ => EMBED_HELLO.to_owned().into_response(),
    }
}

pub async fn web_paste(headers: HeaderMap) -> impl IntoResponse {
    use ClientType::*;

    match ClientType::from(&headers) {
        HTML => WEB_PASTE.to_owned().into_response(),
        NoHtml => HELLO.to_owned().into_response(),
        _ => EMBED_PASTE.to_owned().into_response(),
    }
}

pub async fn not_found(headers: HeaderMap) -> impl IntoResponse {
    use ClientType::*;

    match ClientType::from(&headers) {
        HTML => NOT_FOUND_HTML.to_owned().into_response(),
        NoHtml => "Not found.".into_response(),
        _ => NOT_FOUND_EMBED.to_owned().into_response(),
    }
}

pub fn new_embed(
    title: &str,
    site_name: &str,
    description: &str,
    url: &str,
    limit: usize,
) -> Html<String> {
    let length = description.len();
    let description = description.get(0..limit.min(length)).unwrap_or("");
    Html(format!(
        "
<html>
  <head>
    <meta charset='utf-8' />
    <title>{title}</title>
    <meta name='og:site_name' content='{site_name}' />
    'meta name='author' content='CordlessCoder' />
    <meta
      name='description'
      content='{description}{0}'
    />
    <meta property='og:title' content='{title}' />
    <meta
      content='{description}{0}'
      property='og:description'
    />
    <meta property='og:url' content='{url}' />
    <meta content='#F7768E' data-react-helmet='true' name='theme-color' />
  </head>
</html>",
        if length > limit { "..." } else { "" }
    ))
}

pub fn sanitize_html<'a, S: Into<Cow<'a, str>>>(input: S) -> Cow<'a, str> {
    lazy_static! {
        static ref REGEX: Regex = Regex::new("[<>&]").unwrap();
    }
    let input = input.into();
    let first = REGEX.find(&input);
    let Some(first) = first else {
        return input
    };
    let len = input.len();
    let mut output: Vec<u8> = Vec::with_capacity(len + len / 3);
    output.extend_from_slice(input[0..first.start()].as_bytes());
    let rest = input[first.start()..].bytes();
    for c in rest {
        match c {
            b'<' => output.extend_from_slice(b"&lt;"),
            b'>' => output.extend_from_slice(b"&gt;"),
            b'&' => output.extend_from_slice(b"&amp;"),
            _ => output.push(c),
        }
    }
    Cow::Owned(unsafe { String::from_utf8_unchecked(output) })
}

#[derive(Debug, PartialEq)]
pub enum ClientType {
    Discord,
    Slack,
    Twitter,
    WhatsApp,
    UnknownBot,
    NoHtml,
    HTML,
}

impl From<&HeaderMap> for ClientType {
    fn from(headers: &HeaderMap) -> Self {
        use ClientType::*;
        let Some(h_uagent) = headers.get(HeaderName::from_static("user-agent")) else {
            return NoHtml
        };
        let Ok(uagent) = h_uagent.to_str() else {
            return NoHtml
        };
        [
            (Discord, vec!["Discordbot"]),
            (Twitter, vec!["Twitterbot"]),
            (WhatsApp, vec!["WhatsApp"]),
            (Slack, vec!["Slackbot", "Slack-ImgProxy"]),
            (UnknownBot, vec!["bot"]),
        ]
        .into_iter()
        .find(|(_, header)| header.into_iter().any(|header| uagent.contains(header)))
        .unwrap_or((
            // None of the embed service types matched
            {
                let Some(a) = headers.get(HeaderName::from_static("accept")) else {
                       return NoHtml
                    };
                if !a.to_str().unwrap_or("").contains("html") {
                    return NoHtml;
                }
                HTML
            },
            vec![],
        ))
        .0
    }
}

pub async fn help(headers: HeaderMap) -> impl IntoResponse {
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
        html_to_text(&*data, 80)
    };
    pub static ref WEB_SHORT: Html<String> = Html({
        let mut file = File::open(FILES_DIR.to_owned() + "/WEB_SHORT.html").unwrap();
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();
        data.replace(r"{IP_ADDR}", IP)
    },);
    pub static ref WEB_ANALYTICS: Html<String> = Html({
        let mut file = File::open(FILES_DIR.to_owned() + "/WEB_ANALYTICS.html").unwrap();
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
    pub static ref NOT_FOUND_HTML: Html<String> = Html({
        let mut file = File::open(FILES_DIR.to_owned() + "/NOT_FOUND.html").unwrap();
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();
        data.replace(r"{IP_ADDR}", IP)
    },);
    pub static ref NOT_FOUND_EMBED: Html<String> =
        new_embed("Not Found", "OxiiLink", "Cound not find this item.", IP, 50);
}
