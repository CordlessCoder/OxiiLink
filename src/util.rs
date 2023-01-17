use crate::handlers_paste::{BACKGROUND, FOREGROUND, LOGOFONT};
use crate::state::CurState;
use crate::{StatusCode, UrlPath, FILES_DIR, IP, PASTE_CF, URL_CF};
use axum::extract::State;
use axum::http::header::HeaderName;
use axum::http::HeaderMap;
use axum::response::Html;
use axum::{response::IntoResponse, routing::get_service};
use chrono::{TimeZone, Utc};
use html2text::from_read;
use image::{ImageBuffer, Rgba, RgbaImage};
use imageproc::drawing::{draw_line_segment_mut, draw_text_mut};
use lazy_static::lazy_static;
use memchr::memchr3;
use regex::Regex;
use rocksdb::properties::ESTIMATE_NUM_KEYS;
use rusttype::Scale;
use std::borrow::Cow;
use std::fs::File;
use std::io::Read;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
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

pub async fn get_entries(State(state): State<CurState>) -> Result<impl IntoResponse, StatusCode> {
    let (Some(url_cf), Some(paste_cf)) = (state.db.cf_handle(URL_CF),state.db.cf_handle(PASTE_CF)) else {
            return Err(StatusCode::INTERNAL_SERVER_ERROR)
    };
    let (Ok(Some(url_count)), Ok(Some(paste_count))) = (state.db.property_int_value_cf(&url_cf,ESTIMATE_NUM_KEYS), state.db.property_int_value_cf(&paste_cf,ESTIMATE_NUM_KEYS)) else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR)
    };
    Ok(format!(
        "Total URL Shortened: {url_count}\nTotal pastes hosted: {paste_count}"
    ))
}

pub async fn analytics_paste(
    UrlPath(paste): UrlPath<String>,
    headers: HeaderMap,
    State(state): State<CurState>,
) -> Result<impl IntoResponse, StatusCode> {
    let (paste, ext) = match paste.split_once('.') {
        Some((paste, ext)) => (paste, Some(ext)),
        None => (paste.as_str(), None),
    };
    let Some(entry) = state.get(paste, PASTE_CF)  else {
        return Err(StatusCode::NOT_FOUND)};
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
            &format!(
                "{}/i/{paste}{}",
                IP,
                ext.map(|x| format!(".{x}")).unwrap_or_default()
            ),
        )
        .into_response()),
    }
}
pub async fn analytics_url(
    UrlPath(short): UrlPath<String>,
    headers: HeaderMap,
    State(state): State<CurState>,
) -> Result<impl IntoResponse, StatusCode> {
    let Some(entry) = state.get(&short, URL_CF)  else {
        return Err(StatusCode::NOT_FOUND)};
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
            "",
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
        HTML => HTML_NOT_FOUND.to_owned().into_response(),
        NoHtml => "Not Found.".into_response(),
        _ => EMBED_NOT_FOUND.to_owned().into_response(),
    }
}

pub fn new_embed(
    title: &str,
    site_name: &str,
    description: &str,
    url: &str,
    limit: usize,
    image: &str,
) -> Html<String> {
    let length = description.len();
    let description = description.get(0..limit.min(length)).unwrap_or("");
    Html(format!(
        "<meta name='twitter:card' content='summary_large_image' /><meta name='twitter:image:src' content='{image}' /><meta name='twitter:title' content='{title}' /><meta name='twitter:description' content='{description}{0}' /><meta property='og:image' content='{image}' /><meta property='og:image:alt' content='{description}{0}' /><meta property='og:site_name' content='{site_name}' /><meta property='og:type' content='object' /><meta property='og:title' content='{title}' /><meta property='og:url' content='{url}' /><meta property='og:description' content='{description}{0}' />
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

// pub fn sanitize_html<'a, S: Into<Cow<'a, str>>>(input: S) -> Cow<'a, str> {
//     let input = input.into();
//     let Some(first) = memchr3(b'<', b'>', b'&', input.as_bytes()) else {
//     return input
//     };
//     let len = input.len();
//     let mut output: Vec<u8> = Vec::with_capacity(len + len / 2);
//     output.extend_from_slice(input[0..first].as_bytes());
//     let rest = input[first..].as_bytes();
//     let mut matches = memchr3_iter(b'<', b'>', b'&', rest);
//     let mut nmatch = matches.next();
//     let mut i = 0;
//     while i < rest.len() {
//         match nmatch {
//             Some(n) if n == i => {
//                 // If the current character was the next in the matches
//                 nmatch = matches.next();
//                 match rest[i] {
//                     b'<' => output.extend_from_slice(b"&lt;"),
//                     b'>' => output.extend_from_slice(b"&gt;"),
//                     b'&' => output.extend_from_slice(b"&amp;"),
//                     c => output.push(c),
//                 }
//             }
//             Some(n) => {
//                 output.extend_from_slice(&rest[i..n]);
//                 i = n;
//                 continue;
//             }
//             None => {
//                 output.extend_from_slice(&rest[i..]);
//                 break;
//             }
//         }
//         i += 1
//     }
//     Cow::Owned(unsafe { String::from_utf8_unchecked(output) })
// }

pub fn sanitize_html<'a, S: Into<Cow<'a, str>>>(input: S) -> Cow<'a, str> {
    let input = input.into();
    let first = memchr3(b'<', b'>', b'&', input.as_bytes());
    let Some(first) = first else {
    return input
    };
    let len = input.len();
    let mut output: Vec<u8> = Vec::with_capacity(len + len / 3);
    output.extend_from_slice(input[0..first].as_bytes());
    let rest = input[first..].bytes();
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
        match headers.get(HeaderName::from_static("user-agent")) {
            Some(h_uagent) => {
                let Ok(uagent) = h_uagent.to_str() else {
                    return NoHtml};
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
            }
            None => NoHtml,
        }
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
    pub static ref SYNTAXSET: SyntaxSet = SyntaxSet::load_defaults_newlines();
    pub static ref THEMESET: ThemeSet = ThemeSet::load_defaults();
    pub static ref THEME: Theme = THEMESET.themes["Solarized (dark)"].clone();
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
    pub static ref EMBED_NOT_FOUND: Html<String> =
        new_embed("Not Found", "OxiiLink", "", IP, 50, "");
    pub static ref HTML_NOT_FOUND: Html<String> = Html({
        let mut file = File::open(FILES_DIR.to_owned() + "/NOT_FOUND.html").unwrap();
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
    pub static ref NOT_FOUND_EMBED: Html<String> = new_embed(
        "Not Found",
        "OxiiLink",
        "Cound not find this item.",
        IP,
        50,
        ""
    );
}
pub fn round(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, radius: (u32, u32, u32, u32)) {
    let (width, height) = img.dimensions();
    assert!(radius.0 + radius.1 <= width);
    assert!(radius.3 + radius.2 <= width);
    assert!(radius.0 + radius.3 <= height);
    assert!(radius.1 + radius.2 <= height);

    // top left
    border_radius(img, radius.0, |x, y| (x - 1, y - 1));
    // top right
    border_radius(img, radius.1, |x, y| (width - x, y - 1));
    // bottom right
    border_radius(img, radius.2, |x, y| (width - x, height - y));
    // bottom left
    border_radius(img, radius.3, |x, y| (x - 1, height - y));
}

fn border_radius(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    r: u32,
    coordinates: impl Fn(u32, u32) -> (u32, u32),
) {
    if r == 0 {
        return;
    }
    let r0 = r;

    // 16x antialiasing: 16x16 grid creates 256 possible shades, great for u8!
    let r = 16 * r;

    let mut x = 0;
    let mut y = r - 1;
    let mut p: i32 = 2 - r as i32;

    // ...

    let mut alpha: u16 = 0;
    let mut skip_draw = true;

    let draw = |img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, alpha, x, y| {
        debug_assert!((1..=256).contains(&alpha));
        let pixel_alpha = &mut img[coordinates(r0 - x, r0 - y)].0[3];
        *pixel_alpha = ((alpha * *pixel_alpha as u16 + 128) / 256) as u8;
    };

    'l: loop {
        // (comments for bottom_right case:)
        // remove contents below current position
        {
            let i = x / 16;
            for j in y / 16 + 1..r0 {
                img[coordinates(r0 - i, r0 - j)].0[3] = 0;
            }
        }
        // remove contents right of current position mirrored
        {
            let j = x / 16;
            for i in y / 16 + 1..r0 {
                img[coordinates(r0 - i, r0 - j)].0[3] = 0;
            }
        }

        // draw when moving to next pixel in x-direction
        if !skip_draw {
            draw(img, alpha, x / 16 - 1, y / 16);
            draw(img, alpha, y / 16, x / 16 - 1);
            alpha = 0;
        }

        for _ in 0..16 {
            skip_draw = false;

            if x >= y {
                break 'l;
            }

            alpha += y as u16 % 16 + 1;
            if p < 0 {
                x += 1;
                p += (2 * x + 2) as i32;
            } else {
                // draw when moving to next pixel in y-direction
                if y % 16 == 0 {
                    draw(img, alpha, x / 16, y / 16);
                    draw(img, alpha, y / 16, x / 16);
                    skip_draw = true;
                    alpha = (x + 1) as u16 % 16 * 16;
                }

                x += 1;
                p -= (2 * (y - x) + 2) as i32;
                y -= 1;
            }
        }
    }

    // one corner pixel left
    if x / 16 == y / 16 {
        // column under current position possibly not yet accounted
        if x == y {
            alpha += y as u16 % 16 + 1;
        }
        let s = y as u16 % 16 + 1;
        let alpha = 2 * alpha - s * s;
        draw(img, alpha, x / 16, y / 16);
    }

    // remove remaining square of content in the corner
    let range = y / 16 + 1..r0;
    for i in range.clone() {
        for j in range.clone() {
            img[coordinates(r0 - i, r0 - j)].0[3] = 0;
        }
    }
}

pub fn create_image(size: (u32, u32), padding: u32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut image = RgbaImage::from_pixel(size.0, size.1, BACKGROUND);
    let radius = 16;
    draw_text_mut(
        &mut image,
        FOREGROUND,
        (size.0 - padding - 330) as i32,
        padding as i32,
        Scale { x: 80.0, y: 80.0 },
        &LOGOFONT,
        "OxiiLink",
    );
    draw_line_segment_mut(
        &mut image,
        (0.0, 80.0),
        (size.0 as f32, 80.0),
        Rgba([65, 72, 104, 255]),
    );
    draw_line_segment_mut(
        &mut image,
        (50.0, 80.0),
        (50.0, size.1 as f32),
        Rgba([65, 72, 104, 255]),
    );
    round(&mut image, (radius, radius, radius, radius));
    image
}
