use axum::body::Bytes;
use axum::http::{header, HeaderMap};
use axum::response::{Html, IntoResponse};

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
    if let Err(_) = state.put(&id, data, PASTE_CF) {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Malformed response from the database",
        ));
    }
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
    if let Some(data) = state.get_bytes(paste.as_bytes(), PASTE_CF) {
        match client {
            HTML => {
                if let Some(ext) = ext {
                    if let Ok(data) = std::str::from_utf8(&data) {
                        // If data is valid UTF-8, return with syntax highlighting
                        let data = r"<html><head>
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
</code></pre>
</body></html>";

                        Ok((StatusCode::OK, Html(data).into_response()))
                    } else {
                        // If data isn't valid UTF-8, return it as plain text without syntax highlighting
                        Ok((
                            StatusCode::OK,
                            ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], data)
                                .into_response(),
                        ))
                    }
                } else {
                    Ok((
                        StatusCode::OK,
                        ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], data)
                            .into_response(),
                    ))
                }
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
                Ok((
                    StatusCode::OK,
                    new_embed(
                        &url,
                        std::str::from_utf8(&data).unwrap_or("Binary paste"),
                        &url,
                        240,
                    )
                    .into_response(),
                ))
            }
        }
    } else {
        Err(StatusCode::NOT_FOUND)
    }
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
    if let Ok(exists) = state.key_exists(&paste, PASTE_CF) {
        if exists {
            Err((StatusCode::CONFLICT, "Paste with this name already exists"))
        } else {
            if let Some(data_trunacted) = data.get(0..(MAX_PASTE_BYTES.min(data.len()))) {
                if let Err(_) = state.put(&paste, data_trunacted, PASTE_CF) {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Malformed response from the database",
                    ));
                }
                Ok((StatusCode::CREATED, format!("{IP}/p/{}", &paste)))
            } else {
                Err((StatusCode::UNPROCESSABLE_ENTITY, "Incorrect request body"))
            }
        }
    } else {
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Malformed response from the database",
        ))
    }
}
