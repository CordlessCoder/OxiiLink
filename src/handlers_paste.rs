use std::io::Cursor;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{header, HeaderMap, HeaderValue};
use axum::response::{Html, IntoResponse};
use chrono::Utc;
use image::{ImageFormat, Rgba};
use imageproc::drawing::{draw_line_segment_mut, draw_text_mut};
use lazy_static::lazy_static;
use rusttype::{Font, Scale};
use syntect::easy::HighlightLines;
use syntect::highlighting::FontStyle;
use syntect::util::LinesWithEndings;

use crate::bot::isbot;
use crate::state::{CurState, Entry};
use crate::syntax::highlight_to_html;
use crate::util::{new_embed, SYNTAXSET, THEME};
use crate::ClientType;
use crate::{id, StatusCode, UrlPath, IP, MAX_PASTE_BYTES, PASTE_CF, PASTE_ID_LENGTH};
use chrono::TimeZone;
pub async fn new_paste(
    State(state): State<CurState>,
    mut data: Bytes,
) -> Result<(StatusCode, String), (StatusCode, &'static str)> {
    let length = data.len();
    if length == 0 {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            "Cannot create paste with an empty body",
        ));
    }
    data.truncate(MAX_PASTE_BYTES);
    let id = id::Id::new(PASTE_ID_LENGTH).into_inner();
    let Ok(_) = state.put(&id, Entry::new(data, 0, 0, false), PASTE_CF) else {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Malformed response from the database",
        ));
    };
    Ok((
        if length <= MAX_PASTE_BYTES {
            StatusCode::CREATED
        } else {
            StatusCode::PARTIAL_CONTENT
        },
        format!("{IP}/{}", unsafe { std::str::from_utf8_unchecked(&id) }),
    ))
}

pub async fn get_paste(
    UrlPath(paste): UrlPath<String>,
    headers: HeaderMap,
    State(state): State<CurState>,
) -> Result<(StatusCode, impl IntoResponse), StatusCode> {
    use ClientType::*;
    let (paste, ext) = match paste.split_once('.') {
        Some((paste, ext)) => (paste, Some(ext)),
        None => (paste.as_str(), None),
    };
    let client = ClientType::from(&headers);
    // no file extension
    let Some(entry) = state.get(paste, PASTE_CF) else {
        return Err(StatusCode::NOT_FOUND)};
    let (mut views, mut scrapes, data) = (entry.views, entry.scrapes, entry.contents);
    if isbot(&headers) {
        scrapes += 1
    } else {
        views += 1
    }
    state
        .put(
            paste,
            Entry::new(data.clone(), views, scrapes, false),
            PASTE_CF,
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let out = match client {
        HTML => {
            let Some(ext) = ext else {
                return Ok((
                    StatusCode::OK,
                    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], data).into_response(),
                ))};
            let Ok(text) = std::str::from_utf8(&data) else {
                // If data isn't valid UTF-8, return it as plain text without syntax highlighting
                return Ok((
                    StatusCode::OK,
                    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], data).into_response(),
                ))};
            // If data is valid UTF-8, return with syntax highlighting
            let Some(syntax) = SYNTAXSET.find_syntax_by_token(ext) else {
                // If data isn't valid UTF-8, return it as plain text without syntax highlighting
                return Ok((
                    StatusCode::OK,
                    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], data).into_response(),
                ))};
            let data = highlight_to_html(
                text,
                &SYNTAXSET,
                syntax,
                "
<div class=\"box\">
				<button onclick=\"window.location.href = window.location.href.slice(0, window.location.href.lastIndexOf('/'))\">New Paste</button>
				<button onclick=\"let loc = window.location.href;window.location.href = loc.slice(0,loc.lastIndexOf('/')) + loc.slice(loc.lastIndexOf('/')).replace('/','#');\">Copy &amp; Edit</button>
				<button onclick=\"let loc = window.location.href;window.location.href = loc.slice(0,loc.lastIndexOf('/')) + '/a' + loc.slice(loc.lastIndexOf('/'));\">Analytics</button>
			</div>
			<div id=\"box_hint\" style=\"display: none;\">
				<div class=\"label\">Save</div>
				<div class=\"shortcut\">control + s</div>
			</div>",
            );
            Ok((StatusCode::OK, Html(data).into_response()))

            //             let data = r"<!DOCTYPE html>
            // <html><head>
            // <link rel='stylesheet' href='resource://content-accessible/plaintext.css' />
            // <link
            // rel='stylesheet'
            // href='/files/github-dark.min.css'
            // />
            // <script src='//cdnjs.cloudflare.com/ajax/libs/highlight.js/11.6.0/highlight.min.js'></script>
            // <script>
            // hljs.highlightAll();
            // </script>
            // </head>
            // <body>
            // <pre><code class='language-"
            //                 .to_string()
            //                 + ext
            //                 + r"'>"
            //                 + &sanitize_html(data)
            //                 + r"
            // </code></pre></body></html>";
        }
        NoHtml => Ok((
            StatusCode::OK,
            ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], data).into_response(),
        )),
        _ => {
            let url = format!("{IP}/{paste}{}", {
                if let Some(ext) = ext {
                    format!(".{ext}")
                } else {
                    "".to_string()
                }
            });
            // let data = sanitize_html(std::str::from_utf8(&data).unwrap_or("Binary paste"))
            //     .replace("'", "");
            // let words = data.get(..35.min(data.len())).unwrap();
            // let mut title = String::with_capacity(64);
            let title = "Paste on OxiiLink";
            // let mut title = words
            //     .split_whitespace()
            //     .rev()
            //     .skip(1)
            //     .fold(String::new(), |acc, x| format!("{x} {acc}"));
            // if title.is_empty() {
            //     title = data.get(..35.min(data.len())).unwrap().to_string() + "...";
            // }
            Ok((
                StatusCode::OK,
                new_embed(
                    title.trim(),
                    "OxiiLink",
                    "", //&data,
                    &url,
                    240,
                    &format!(
                        "{}/i/{paste}{}",
                        IP,
                        ext.map(|x| format!(".{x}")).unwrap_or_default()
                    ),
                )
                .into_response(),
            ))
        }
    };
    out
}

pub async fn delete_paste(
    UrlPath(paste): UrlPath<String>,
    State(state): State<CurState>,
) -> (StatusCode, &'static str) {
    state.cache.remove(&paste).await;
    match state.delete(
        paste
            .split_once('.')
            .map(|(name, _)| name)
            .unwrap_or(&paste),
        PASTE_CF,
    ) {
        Ok(_) => (StatusCode::OK, "Success"),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Malformed response from database.",
        ),
    }
}

pub async fn create_paste(
    UrlPath(paste): UrlPath<String>,
    data: String,
    State(state): State<CurState>,
) -> Result<(StatusCode, String), (StatusCode, &'static str)> {
    let length = paste.len();
    if length > 16 || length <= 1 {
        return Err((StatusCode::BAD_REQUEST, "custom ID out of bounds"));
    }
    let Ok(exists) = state.key_exists(&paste, PASTE_CF) else {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Malformed response from the database",
        ))};
    if exists {
        Err((StatusCode::CONFLICT, "Paste with this name already exists"))
    } else {
        let Some(data_trunacted) = data.get(0..(MAX_PASTE_BYTES.min(data.len()))) else {
            return Err((StatusCode::UNPROCESSABLE_ENTITY, "Incorrect request body"))};
        let Ok(_) = state.put(&paste, Entry::new(data_trunacted, 0, 0, false), PASTE_CF) else {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Malformed response from the database",
            ));
        };
        Ok((StatusCode::CREATED, format!("{IP}/p/{}", &paste)))
    }
}

lazy_static! {
    pub static ref FONT: Font<'static> =
        Font::try_from_bytes(include_bytes!("../assets/LiberationMono-Regular.ttf")).unwrap();
    pub static ref LOGOFONT: Font<'static> = Font::try_from_bytes(include_bytes!(
        "../assets/Fira Code Bold Nerd Font Complete Mono.ttf"
    ))
    .unwrap();
}

pub const BACKGROUND: Rgba<u8> = Rgba([17, 18, 29, 255]);
pub const FOREGROUND: Rgba<u8> = Rgba([247, 118, 142, 255]);
pub const SIZE: (i32, i32) = (1200, 600);

pub async fn paste_image(
    UrlPath(pasteurl): UrlPath<String>,
    // headers: HeaderMap,
    State(state): State<CurState>,
) -> Result<(StatusCode, impl IntoResponse), StatusCode> {
    // use ClientType::*;
    // if isbot(&headers) {
    //     return Err(StatusCode::FORBIDDEN);
    // }
    // let client = ClientType::from(&headers);
    let (paste, ext) = match pasteurl.split_once('.') {
        Some((paste, ext)) => (paste, Some(ext)),
        None => (pasteurl.as_str(), None),
    };
    // no file extension

    if paste.as_bytes().len() > PASTE_ID_LENGTH {
        return Err(StatusCode::NOT_FOUND);
    }
    let Some((data, created_at)) = state.get(paste, PASTE_CF).map(|x|(x.contents, x.creationdate)) else {
        return Err(StatusCode::NOT_FOUND)};
    let data = if let Ok(data) = std::str::from_utf8(&data) {
        data
    } else {
        "Binary paste"
    };
    let syntax = if let Some(Some(syntax)) = ext.map(|ext| SYNTAXSET.find_syntax_by_token(ext)) {
        syntax
    } else {
        SYNTAXSET
            .find_syntax_by_first_line(data)
            .unwrap_or(SYNTAXSET.find_syntax_plain_text())
    };
    if let Some(cached) = state.cache.get(&format!("{paste}{}", syntax.name)) {
        let mut response = cached.value().clone().into_response();
        let _ = response
            .headers_mut()
            .insert("Content-type", HeaderValue::from_static("image/png"));
        return Ok((StatusCode::OK, response));
    }

    let size = SIZE;
    let padding = 5;
    let mut image = *state.image.clone();

    draw_text_mut(
        &mut image,
        Rgba([169, 177, 214, 255]),
        padding as i32,
        padding as i32,
        Scale { x: 50.0, y: 50.0 },
        &LOGOFONT,
        &syntax.name,
    );
    draw_text_mut(
        &mut image,
        Rgba([169, 177, 214, 255]),
        padding as i32,
        padding as i32 + 45,
        Scale { x: 30.0, y: 30.0 },
        &FONT,
        &format!(
            "Created: {}",
            Utc.timestamp_opt(created_at, 0)
                .unwrap()
                .format("%H:%M %d/%m/%Y")
        ),
    );
    let gutter = 48;
    let mut cursor = Cursor::new(Vec::with_capacity(image.len()));
    {
        // Scope for working with HighlightLines, for some reason everything breaks if
        // HighlightLines is in the main scope
        let mut h = HighlightLines::new(syntax, &THEME);
        let mut lines = LinesWithEndings::from(data)
            .filter_map(|line| h.highlight_line(line, &SYNTAXSET).ok())
            .enumerate()
            .peekable();
        let scale = Scale { x: 42.0, y: 42.0 };
        let top_padding = 80;
        let correction = (0.53, 1.0);
        let mut y: f32 = (padding + top_padding) as f32;
        let mut empty = false;
        let char_width = scale.x * correction.0;
        while let Some((nr, line)) = lines.next() {
            if empty {
                y -= scale.y;
            }
            empty = false;
            let mut x: f32 = (padding + gutter) as f32;
            // Draw line number
            draw_text_mut(
                &mut image,
                FOREGROUND,
                padding,
                y as i32,
                scale,
                &FONT,
                &(nr + 1).to_string(),
            );
            draw_line_segment_mut(
                &mut image,
                (0.0, y + scale.y),
                (gutter as f32, y + scale.y),
                Rgba([65, 72, 104, 255]),
            );
            for (style, word) in line {
                let chars_left = ((size.0 - (x as i32 + padding)) as f32 / char_width) as usize;
                let mut offset = 0;
                let word = word.replace('\n', "");
                if chars_left < word.len() {
                    if chars_left > 2 {
                        draw_text_mut(
                            &mut image,
                            Rgba([
                                style.foreground.r,
                                style.foreground.g,
                                style.foreground.b,
                                style.foreground.a,
                            ]),
                            x as i32,
                            y as i32,
                            scale,
                            match style.font_style {
                                FontStyle::BOLD => &FONT,
                                FontStyle::ITALIC => &FONT,
                                _ => &FONT,
                            },
                            &word[..word.ceil_char_boundary(chars_left)],
                        );
                        draw_text_mut(
                            &mut image,
                            Rgba([
                                style.foreground.r,
                                style.foreground.g,
                                style.foreground.b,
                                style.foreground.a,
                            ]),
                            padding + gutter,
                            (y + scale.y) as i32,
                            scale,
                            match style.font_style {
                                FontStyle::BOLD => &FONT,
                                FontStyle::ITALIC => &FONT,
                                _ => &FONT,
                            },
                            &word[word.floor_char_boundary(chars_left)..],
                        );
                        offset = (char_width * (word.len() - chars_left) as f32) as i32;
                    }

                    x = (padding + gutter + offset) as f32;
                    y += scale.y;
                    if word.trim().len() == 0 {
                        if empty {
                            y -= scale.y;
                        }
                        empty = true;
                        continue;
                    }
                }

                empty = false;
                draw_text_mut(
                    &mut image,
                    Rgba([
                        style.foreground.r,
                        style.foreground.g,
                        style.foreground.b,
                        style.foreground.a,
                    ]),
                    x as i32,
                    y as i32,
                    scale,
                    match style.font_style {
                        FontStyle::BOLD => &FONT,
                        FontStyle::ITALIC => &FONT,
                        _ => &FONT,
                    },
                    &word,
                );
                x += char_width * word.len() as f32;
            }
            y += scale.y;
            if y as i32 + padding + (scale.y * 0.8) as i32 > size.1 {
                break;
            }
        }
    };

    image
        .write_to(&mut cursor, ImageFormat::Png)
        .expect("SOMEHOW failed to write to a memory-backed cursor. This is bad.");
    let image = cursor.into_inner();
    state
        .cache
        .insert(
            format!("{paste}{}", syntax.name),
            image.clone(),
            image.len() as i64,
        )
        .await;

    let mut response = image.into_response();
    let _ = response
        .headers_mut()
        .insert("Content-type", HeaderValue::from_static("image/png"));
    Ok((StatusCode::OK, response))
}
