use warp::http::StatusCode;
use warp::reject::Rejection;
use warp::reply;
use warp::Reply;

#[derive(Debug)]
pub struct InvalidBase64;

impl warp::reject::Reject for InvalidBase64 {}

#[derive(Debug)]
pub struct NotFound;

impl warp::reject::Reject for NotFound {}

pub async fn handle_rejection(err: Rejection) -> Result<impl Reply, std::convert::Infallible> {
    if err.is_not_found() {
        Ok(reply::with_status("NOT_FOUND", StatusCode::NOT_FOUND))
    } else if let Some(_) = err.find::<InvalidBase64>() {
        Ok(reply::with_status("BAD_REQUEST", StatusCode::BAD_REQUEST))
    } else if let Some(_) = err.find::<NotFound>() {
        Ok(reply::with_status("NOT_FOUND", StatusCode::NOT_FOUND))
    } else {
        eprintln!("unhandled rejection: {:?}", err);
        Ok(reply::with_status(
            "INTERNAL_SERVER_ERROR",
            StatusCode::INTERNAL_SERVER_ERROR,
        ))
    }
}
