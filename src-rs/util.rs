use tracing::error;
use vercel_runtime::Error;

pub fn internal_server_error(err: Box<dyn std::error::Error>) -> Error {
    error!("{err:?}");
    "Internal Server Error".into()
}
