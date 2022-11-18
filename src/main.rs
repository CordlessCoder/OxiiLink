#![allow(dead_code)]
use axum::response::Redirect;
use axum::{
    extract::{Extension, Path as UrlPath},
    http::StatusCode,
    routing::{delete, get, post},
    Router,
};
use rocksdb::{self, DB};
use std::net::SocketAddr;
use std::sync::Arc;
use tower::ServiceBuilder;
use url::Url;
use util::*;

mod handlers_paste;
mod handlers_shorten;
mod id;
mod state;
mod util;
use handlers_paste::*;
use handlers_shorten::*;
use state::*;

// TODO: move this to a configuration file and add argument overrides
static PASTE_ID_LENGTH: usize = 3;
static URL_ID_LENGTH: usize = 3;
static IP: &str = "https://roman.vm.net.ua";
static SOCKETADDR: ([u8; 4], u16) = ([127, 0, 0, 1], 3000);
static PATH: &str = "/tmp/test";
static FILES_DIR: &str = "../files";
static URL_CF: &str = "URL";
static PASTE_CF: &str = "PASTE";
static MAX_PASTE_BYTES: usize = 1024 * 128;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cache = rocksdb::Cache::new_lru_cache(128)?;
    let db = {
        let mut opts = rocksdb::Options::default();
        opts.set_compression_type(rocksdb::DBCompressionType::Lz4);
        opts.create_missing_column_families(true);
        opts.set_row_cache(&cache);
        opts.create_if_missing(true);
        opts.set_max_background_jobs(4);
        Arc::new(DB::open_cf_descriptors(
            &opts,
            PATH,
            util::make_descriptors(rocksdb::Options::default(), vec![URL_CF, PASTE_CF]),
        )?)
    };

    // Configure tracing if desired
    // construct a subscriber that prints formatted traces to stdout
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    // use that subscriber to process traces emitted after this point
    tracing::subscriber::set_global_default(subscriber).unwrap();
    let state = State { db, cache };
    // let config = RustlsConfig::from_pem_file(
    //     "../private/certificate.pem",
    //     "../private/private.key.pem"
    // )
    // .await?;
    let app = Router::new()
        // .route("/list", get(list))
        .route("/w", get(web_short))
        .route("/p", get(web_paste))
        .route("/p/:url", get(get_paste))
        .nest("/files/", util::serve())
        // .route("/p/:url", post(create_paste))
        .route("/p/:url", delete(delete_paste))
        .route("/p", post(new_paste))
        .route("/:url", get(get_url))
        // .route("/:url", post(create_url))
        .route("/:url", delete(delete_url))
        .route("/", post(shorten_url))
        .route("/", get(util::root))
        .layer(
            ServiceBuilder::new()
                // .layer(TraceLayer::new_for_http())
                .layer(Extension(state)),
        );

    let addr = SocketAddr::from(SOCKETADDR);
    println!("Listening on {}", addr);
    // axum_server::bind_rustls(addr, config)
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await?;
    Ok(())
}
