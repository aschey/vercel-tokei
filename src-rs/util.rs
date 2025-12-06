use tracing::error;
use vercel_runtime::Error;

pub fn internal_server_error(err: Error) -> Error {
    error!("{err:?}");
    "Internal Server Error".into()
}
