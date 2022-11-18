use crate::{id, Extension, Redirect, State, StatusCode, Url, UrlPath, IP, URL_CF, URL_ID_LENGTH};
use lazy_static::lazy_static;

pub async fn get_url(
    UrlPath(short): UrlPath<String>,
    Extension(state): Extension<State>,
) -> Result<Redirect, StatusCode> {
    if let Some(full_url) = state.get(short.as_bytes(), URL_CF) {
        Ok(Redirect::to(&full_url))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

pub async fn delete_url(
    UrlPath(short): UrlPath<String>,
    Extension(state): Extension<State>,
) -> StatusCode {
    match state.delete(short, URL_CF) {
        Ok(_) => StatusCode::OK,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

pub async fn create_url(
    UrlPath(short): UrlPath<String>,
    url: String,
    Extension(state): Extension<State>,
) -> Result<(StatusCode, String), (StatusCode, &'static str)> {
    let length = short.len();
    if length > 16 || length <= 1 {
        return Err((StatusCode::BAD_REQUEST, "custom ID length out of bounds"));
    }
    if let Ok(exists) = state.key_exists(&short, URL_CF) {
        if exists {
            Err((
                StatusCode::NOT_MODIFIED,
                "A shortened URL with this ID already exists",
            ))
        } else {
            let parsed_url = Url::parse(&url).map_err(|_err| {
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Does this look like a URL to you?",
                )
            })?;
            let scheme = parsed_url.scheme();
            if parsed_url.username() != "" || scheme != "http" && scheme != "https" {
                return Err((
                    StatusCode::UNSUPPORTED_MEDIA_TYPE,
                    "Cannot shorten this URL",
                ));
            };
            match state.put(&short, parsed_url.to_string(), URL_CF) {
                Ok(_) => Ok((StatusCode::OK, format!("{IP}/{short}\n"))),
                Err(_) => Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Malformed response from database",
                )),
            }
        }
    } else {
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Malformed response from database",
        ))
    }
}

pub async fn shorten_url(
    url: String,
    Extension(state): Extension<State>,
) -> Result<(StatusCode, String), (StatusCode, &'static str)> {
    let parsed_url = Url::parse(&url).map_err(|_err| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            "Does this look like a URL to you?",
        )
    })?;
    let scheme = parsed_url.scheme();
    if parsed_url.username() != ""
        || scheme != "http" && scheme != "https"
        || parsed_url.host_str().is_none()
        || parsed_url.host_str().unwrap() == IP_HOST.as_str()
    {
        return Err((
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Cannot shorten this URL",
        ));
    }

    let id = id::Id::new(URL_ID_LENGTH).into_inner();
    state
        .put(&id, parsed_url.to_string(), URL_CF)
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Malformed response from database",
            )
        })?;
    Ok((
        StatusCode::CREATED,
        format!("{IP}/{}", unsafe {
            std::str::from_utf8_unchecked(&id) // unsafe used here as the id has to be correct UTF-8 as
                                               // we just generated it
        }),
    ))
}

lazy_static! {
    pub static ref IP_HOST: String = Url::parse(IP).unwrap().host_str().unwrap().to_string();
}
