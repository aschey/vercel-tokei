use std::time::Duration;

use eyre::eyre;
use http::Response;
use vercel_runtime::{Error, Request, ResponseBody, service_fn};
use vercel_tokei::util::internal_server_error;

const SECONDS_IN_MINUTE: u64 = 60;

#[cached::proc_macro::cached(
    name = "CACHE",
    result = true,
    with_cached_flag = true,
    ty = "cached::TimedSizedCache<String, cached::Return<String>>",
    create = "{ cached::TimedSizedCache::with_size_and_lifespan(1, Duration::from_secs(15 * \
              SECONDS_IN_MINUTE)) }",
    convert = r#"{ url.to_string() }"#
)]
async fn fetch_readme(url: &str) -> Result<cached::Return<String>, Error> {
    let res = reqwest::get(url).await?;
    let text = res.text().await?;
    Ok(cached::Return::new(text))
}

async fn handler(_req: Request) -> Result<Response<ResponseBody>, Error> {
    let text = fetch_readme("https://raw.githubusercontent.com/aschey/vercel-tokei/main/README.md")
        .await
        .map_err(internal_server_error)?;
    Response::builder()
        .body(
            markdown::to_html_with_options(&text, &markdown::Options::gfm())
                .map_err(|e| eyre!("error parsing markdown: {e:?}"))?
                .into(),
        )
        .map_err(|e| internal_server_error(Box::new(e)))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt().with_ansi(false).init();
    vercel_runtime::run(service_fn(handler)).await
}
