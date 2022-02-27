use redis::aio::ConnectionManager;
use rust_embed::RustEmbed;
use std::net::IpAddr;
use structopt::StructOpt;
use warp::filters::BoxedFilter;
use warp::reject::Rejection;
use warp::Filter;

// use tracing::error;

use crypto_part::{random_bytes, Key};

mod errors;
use errors::{handle_rejection, InvalidBase64, NotFound};

#[derive(RustEmbed)]
#[folder = "../frontend/dist"]
struct Data;

#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
struct Opt {
    /// Activate debug mode
    #[structopt(
        short,
        long,
        env = "SECRET_SHARE_REDIS_URL",
        default_value = "redis://localhost:6379"
    )]
    redis: redis::ConnectionInfo,
    #[structopt(short, long, env = "SECRET_SHARE_ADDRESS", default_value = "::1")]
    address: IpAddr,
    #[structopt(short, long, env = "SECRET_SHARE_PORT", default_value = "3030")]
    port: u16,
}

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();
    pretty_env_logger::init();

    let client = redis::Client::open(opt.redis).unwrap();
    let conn = client.get_tokio_connection_manager().await.unwrap();

    let api = getter(conn.clone()).or(setter(conn));
    let data_serve = warp_embed::embed(&Data);

    let endpoints = api
        .or(data_serve)
        .recover(handle_rejection)
        .with(warp::trace::request());

    warp::serve(endpoints).run((opt.address, opt.port)).await;
}

fn getter(con_manager: ConnectionManager) -> BoxedFilter<(String,)> {
    warp::get()
        .and(warp::path!("api" / String))
        .and(warp::any().map(move || con_manager.clone()).boxed())
        .and_then(move |path: String, conn| redis_get(conn, path.clone()))
        .and_then(decrypt)
        .boxed()
}

fn setter(con_manager: ConnectionManager) -> BoxedFilter<(String,)> {
    warp::post()
        .and(warp::path!("api" / String))
        .and(warp::body::content_length_limit(1024 * 32))
        .and(warp::body::bytes())
        .and_then(encrypt)
        .and(warp::any().map(move || con_manager.clone()).boxed())
        .and_then(move |(id, secret): (String, String), conn| redis_set(conn, id.clone(), secret))
        .boxed()
}

async fn redis_get(mut con_manager: ConnectionManager, path: String) -> Result<String, Rejection> {
    let out: (String, usize) = redis::pipe()
        .atomic()
        .getset(
            &path,
            base64::encode_config(random_bytes(), base64::URL_SAFE_NO_PAD),
        )
        .del(&path)
        .query_async(&mut con_manager)
        .await
        .map_err(|_| NotFound)?;

    Ok(out.0)
}

async fn redis_set(
    mut con_manager: ConnectionManager,
    path: String,
    secret: String,
) -> Result<String, Rejection> {
    let out: String = redis::Cmd::set_ex(&path, secret, 60 * 60 * 24)
        .query_async(&mut con_manager)
        .await
        .map_err(|_| NotFound)?;

    Ok(out)
}

async fn decrypt(txt: String) -> Result<String, Rejection> {
    let bytes = base64::decode_config(txt, base64::URL_SAFE).map_err(|_| NotFound)?;
    let mut out_bytes = Vec::new();
    let secret_key = secret_key();
    crypto_part::decode(&bytes[..], &mut out_bytes, &secret_key).map_err(|_| NotFound)?;

    Ok(base64::encode_config(out_bytes, base64::URL_SAFE))
}

async fn encrypt(
    id: String,
    secret: warp::hyper::body::Bytes,
) -> Result<(String, String), Rejection> {
    let mut secret_slice = &secret[..];
    let decoder = base64::read::DecoderReader::new(&mut secret_slice, base64::URL_SAFE);

    let mut out_bytes = Vec::new();
    let secret_key = secret_key();
    crypto_part::encode(decoder, &mut out_bytes, &secret_key).map_err(|_| InvalidBase64)?;
    let encoded_secret = base64::encode_config(out_bytes, base64::URL_SAFE);

    Ok((id, encoded_secret))
}

fn secret_key() -> Key {
    let raw_str = std::env::var("SECRET_SHARE_SECREY_KEY").unwrap_or(String::from("super secret"));
    Key::from(raw_str)
}

// fn hello_wrapper<F, T>(
//     filter: F,
// ) -> BoxedFilter<(T,)>
// // ) -> impl Filter<Extract = (T,)> + Clone + Send + Sync + 'static
// where
//     F: Filter<Extract = (T,), Error = Rejection> + Clone + Send + Sync + 'static,
//     F::Extract: warp::Reply,
// {
//     warp::any()
//         .map(|| {
//             error!("hallo");
//         })
//         .untuple_one()
//         .and(filter)
//         .boxed()
// }
