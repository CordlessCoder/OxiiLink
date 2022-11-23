use axum::body::Bytes;
use axum::http::{header, HeaderMap};
use axum::response::{Html, IntoResponse};

use crate::bot::isbot;
use crate::state::Entry;
use crate::util::{new_embed, sanitize_html};
use crate::ClientType;
use crate::{
    id, Extension, State, StatusCode, UrlPath, IP, MAX_PASTE_BYTES, PASTE_CF, PASTE_ID_LENGTH,
};

pub async fn new_paste(
    mut data: Bytes,
    Extension(state): Extension<State>,
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
    Extension(state): Extension<State>,
) -> Result<(StatusCode, impl IntoResponse), StatusCode> {
    use ClientType::*;
    let (paste, ext) = match paste.split_once('.') {
        Some((paste, ext)) => (paste, Some(ext)),
        None => (paste.as_str(), None),
    };
    let client = ClientType::from(&headers);
    // no file extension
    let Some(entry) = state.get(paste, PASTE_CF) else {
        return Err(StatusCode::NOT_FOUND)
    };
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
                    // If there is no file extension, return data as plain text without syntax
                    // highlighting
                    return Ok((
                        StatusCode::OK,
                        ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], data)
                            .into_response(),
                    ))};
            let Ok(data) = std::str::from_utf8(&data) else {
                    // If data isn't valid UTF-8, return it as plain text without syntax highlighting
                    return Ok((
                        StatusCode::OK,
                        ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], data)
                            .into_response(),
                    ))};
            // If data is valid UTF-8, return with syntax highlighting
            let data = r"<!DOCTYPE html>
<html><head>
<link rel='stylesheet' href='resource://content-accessible/plaintext.css' />
<link
rel='stylesheet'
href='/files/github-dark.min.css'
/>
<script src='//cdnjs.cloudflare.com/ajax/libs/highlight.js/11.6.0/highlight.min.js'></script>
<script>
hljs.highlightAll();
</script>
</head>
<body>
<pre><code class='language-"
                .to_string()
                + ext
                + r"'>"
                + &sanitize_html(data)
                + r"
</code></pre></body></html>";

            Ok((StatusCode::OK, Html(data).into_response()))
        }
        NoHtml => Ok((
            StatusCode::OK,
            ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], data).into_response(),
        )),
        _ => {
            let url = format!("{IP}/{paste}{}", {
                ext.map(|ext| format!(".{ext}")).unwrap_or_default()
            });
            let data = sanitize_html(
                std::str::from_utf8(&data)
                    .map(|x| x.replace('\'', ""))
                    .unwrap_or("Binary paste".to_string()),
            );
            let words = data.get(..35.min(data.len())).unwrap();
            let mut title = words
                .split_whitespace()
                .rev()
                .skip(1)
                .fold(String::new(), |acc, x| format!("{x} {acc}"));
            if title.is_empty() {
                title = data.get(..35.min(data.len())).unwrap().to_string();
            }
            Ok((
                StatusCode::OK,
                new_embed(title.trim(), "OxiiLink", &data, &url, 240).into_response(),
            ))
        }
    };
    out
}

pub async fn delete_paste(
    UrlPath(paste): UrlPath<String>,
    Extension(state): Extension<State>,
) -> (StatusCode, &'static str) {
    match state.delete(paste, PASTE_CF) {
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
    Extension(state): Extension<State>,
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
            return Err((StatusCode::UNPROCESSABLE_ENTITY, "Incorrect request body"))
        };
        let Ok(_) = state.put(&paste, Entry::new(data_trunacted, 0, 0, false), PASTE_CF) else {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Malformed response from the database",
            ));
        };
        Ok((StatusCode::CREATED, format!("{IP}/p/{}", &paste)))
    }
}
