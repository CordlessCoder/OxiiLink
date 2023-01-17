#![allow(dead_code)]
use axum::response::Redirect;
use axum::{
    extract::Path as UrlPath,
    http::StatusCode,
    routing::{delete, get, post},
    Router,
};
use clap::Parser;
use rocksdb::{self, DB};
use std::net::SocketAddr;
use std::sync::Arc;
use stretto::AsyncCache;
use url::Url;

mod bot;
mod cli;
mod handlers_paste;
mod handlers_shorten;
mod id;
mod state;
mod syntax;
mod util;
use cli::Cli;
use handlers_paste::*;
use handlers_shorten::*;
use state::*;
use util::*;

// TODO: move this to a configuration file and add argument overrides
const PASTE_ID_LENGTH: usize = 3;
const URL_ID_LENGTH: usize = 3;
static IP: &str = "https://oxlink.dev";
static SOCKETADDR: ([u8; 4], u16) = ([127, 0, 0, 1], 3000);
static PATH: &str = "db";
static FILES_DIR: &str = "files";
static URL_CF: &str = "URL";
static PASTE_CF: &str = "PASTE";
static MAX_PASTE_BYTES: usize = 1024 * 128;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Cli::parse();
    let db_cache = rocksdb::Cache::new_lru_cache(128)?;
    let db = {
        let mut opts = rocksdb::Options::default();
        opts.set_compression_type(rocksdb::DBCompressionType::Lz4);
        opts.create_missing_column_families(true);
        opts.set_row_cache(&db_cache);
        opts.create_if_missing(true);
        // opts.set_merge_operator_associative("increment", incr_merge);
        opts.set_max_background_jobs(4);
        Arc::new(DB::open_cf_descriptors(
            &opts,
            PATH,
            util::make_descriptors(rocksdb::Options::default(), vec![URL_CF, PASTE_CF]),
        )?)
    };
    let cache = AsyncCache::new(1000, 1024 * 1024 * 50, tokio::spawn)
        .expect("Failed to initialize AsyncCache");

    // Configure tracing if desired
    // construct a subscriber that prints formatted traces to stdout
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    // use that subscriber to process traces emitted after this point
    tracing::subscriber::set_global_default(subscriber).unwrap();
    let image = create_image((1000, 500), 5);
    let state = CurState {
        image: Box::new(image),
        db,
        db_cache,
        cache,
    };
    // let config = RustlsConfig::from_pem_file(
    //     "../private/certificate.pem",
    //     "../private/private.key.pem"
    // )
    // .await?;
    let app = Router::new()
        // .route("/list", get(list))
        .route("/", get(web_paste))
        .route("/status", get(status))
        .route("/nothing/", get(not_found))
        .route("/nothing", get(not_found))
        .route("/count", get(get_entries))
        .route("/a/:paste", get(analytics_paste))
        .route("/a/s/:url", get(analytics_url))
        .route("/a", get(web_analytics))
        .route("/a/", get(web_analytics))
        .route("/:paste", get(get_paste))
        .route("/i/:paste", get(paste_image))
        // .route("/p/:paste", post(create_paste))
        .route("/:paste", delete(delete_paste))
        .nest_service("/files/", util::serve())
        .route("/", post(new_paste))
        .route("/help/", get(util::help))
        .route("/help", get(util::help))
        .route("/s/:url", get(get_url))
        // .route("/:url", post(create_url))
        .route("/s/:url", delete(delete_url))
        .route("/s/", post(shorten_url))
        .route("/s", post(shorten_url))
        .route("/s/", get(web_short))
        .route("/s", get(web_short))
        .with_state(state);

    let addr = SocketAddr::from(SOCKETADDR);
    println!("Listening on {}", addr);
    // axum_server::bind_rustls(addr, config)
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await?;
    Ok(())
}
pub async fn status() -> &'static str {
    "Ok!"
}
