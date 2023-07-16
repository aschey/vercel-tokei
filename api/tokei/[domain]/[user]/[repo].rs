use cached::Cached;
use eyre::Context;
use git2::{Direction, Remote, Repository};
use gix::{interrupt, progress, remote::fetch};
use http::{Method, StatusCode};
use rsbadges::Badge;
use std::collections::HashMap;
use tempfile::TempDir;
use tokei::{Config, Language, Languages};
use tracing::{error, info};
use url::Url;
use vercel_runtime::{Body, Error, Request, Response};

use vercel_tokei::{content_type::ContentType, settings::Settings};

const BILLION: usize = 1_000_000_000;
const MILLION: usize = 1_000_000;
const THOUSAND: usize = 1_000;
const DAY_IN_SECONDS: u64 = 24 * 60 * 60;
const REVALIDATE_FACTOR: u32 = 5;

async fn handler(req: Request) -> Result<Response<Body>, Error> {
    tokio::task::spawn_blocking(|| handle_request(req))
        .await
        .with_context(|| "Task failed to run")?
}

fn handle_request(req: Request) -> Result<Response<Body>, Error> {
    // For health checks
    if req.method() == Method::HEAD {
        return Ok(Response::new("".into()));
    }

    let parsed_url =
        Url::parse(&req.uri().to_string()).map_err(|e| internal_server_error(Box::new(e)))?;
    let hash_query: HashMap<_, _> = parsed_url.query_pairs().collect();

    info!("Query pairs: {hash_query:?}");

    let settings = match Settings::from_query(&hash_query) {
        Ok(settings) => settings,
        Err(e) => return bad_request(e.to_string()),
    };

    let (domain, user, repo) = (
        hash_query
            .get("domain")
            .ok_or_else(|| internal_server_error("domain missing".into()))?,
        hash_query
            .get("user")
            .ok_or_else(|| internal_server_error("user missing".into()))?,
        hash_query
            .get("repo")
            .ok_or_else(|| internal_server_error("repo missing".into()))?,
    );
    let mut domain = percent_encoding::percent_decode_str(domain)
        .decode_utf8()
        .wrap_err_with(|| "Error decoding domain")?;

    if !domain.contains('.') {
        domain += ".com";
    }

    let url = format!("https://{}/{}/{}", domain, user, repo);
    info!("Getting info for {url}");

    let mut repo = match Remote::create_detached(&url[..]) {
        Ok(repo) => repo,
        Err(e) => return bad_request(e.to_string()),
    };

    if let Err(e) = repo.connect(Direction::Fetch) {
        return bad_request(format!("Error connecting to repository: {}", e));
    }

    let repo_list = match repo.list() {
        Ok(list) => list,
        Err(e) => return bad_request(format!("Error listing repo contents: {}", e)),
    };

    let sha = match repo_list.first() {
        Some(sha) => sha.oid().to_string(),
        None => return bad_request("Repo contains no refs".to_string()),
    };
    info!("Repo sha: {sha:?}");

    if let Some(badge) = CACHE
        .lock()
        .expect("Cache mutex poisoned")
        .cache_get(&repo_identifier(&url, &sha))
    {
        info!("Serving from cache");
        return match make_badge(&settings, badge) {
            Ok(badge) => build_response(badge, &settings),
            Err(e) => bad_request(e.to_string()),
        };
    }

    let stats = get_statistics(&url, &sha)
        .wrap_err_with(|| "Error getting statistics")?
        .value;

    match make_badge(&settings, &stats).map_err(internal_server_error) {
        Ok(badge) => build_response(badge, &settings),
        Err(e) => bad_request(e.to_string()),
    }
}

fn internal_server_error(err: Box<dyn std::error::Error>) -> Error {
    error!("{err:?}");
    "Internal Server Error".into()
}

fn build_response(badge: String, settings: &Settings) -> Result<Response<Body>, Error> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", settings.content_type.response_type())
        .header(
            "Cache-Control",
            &format!(
                "s-maxage={}, stale-while-revalidate={}",
                settings.cache_seconds,
                settings.cache_seconds * REVALIDATE_FACTOR
            ),
        )
        .body(badge.into())
        .map_err(|e| internal_server_error(Box::new(e)))
}

fn bad_request(body: String) -> Result<Response<Body>, Error> {
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(body.into())
        .map_err(|e| internal_server_error(Box::new(e)))
}

fn repo_identifier(url: &str, sha: &str) -> String {
    format!("{}#{}", url, sha)
}

fn trim_and_float(num: usize, trim: usize) -> f64 {
    (num as f64) / (trim as f64)
}

fn make_badge(settings: &Settings, stats: &Language) -> Result<String, Box<dyn std::error::Error>> {
    if settings.content_type == ContentType::Json {
        return Ok(serde_json::to_string(&stats)?);
    }

    let amount = settings.category.stats(stats);
    let label = match &settings.label {
        Some(label) => label.as_str(),
        None => settings.category.description(),
    };

    let amount = if amount >= BILLION {
        format!("{:.1}B", trim_and_float(amount, BILLION))
    } else if amount >= MILLION {
        format!("{:.1}M", trim_and_float(amount, MILLION))
    } else if amount >= THOUSAND {
        format!("{:.1}K", trim_and_float(amount, THOUSAND))
    } else {
        amount.to_string()
    };

    let badge = Badge {
        label_text: label.to_owned(),
        label_color: settings.theme.label_color.to_string(),
        msg_text: amount.clone(),
        msg_color: settings.theme.color.to_string(),
        logo: settings.logo.clone().unwrap_or_default(),
        // data urls can't be embedded
        embed_logo: settings
            .logo
            .as_ref()
            .map(|l| !l.starts_with("data:"))
            .unwrap_or(false),
        msg_title: amount,
        label_title: label.to_owned(),
        use_logo_as_label: settings.logo_as_label,
        ..Badge::default()
    };

    let badge_style = settings.theme.style.to_badge_style(badge);
    Ok(badge_style.generate_svg()?)
}

#[cached::proc_macro::cached(
    name = "CACHE",
    result = true,
    with_cached_flag = true,
    type = "cached::TimedSizedCache<String, cached::Return<Language>>",
    create = "{ cached::TimedSizedCache::with_size_and_lifespan(1000, DAY_IN_SECONDS) }",
    convert = r#"{ repo_identifier(url, _sha) }"#
)]
fn get_statistics(url: &str, _sha: &str) -> eyre::Result<cached::Return<Language>> {
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir
        .path()
        .to_str()
        .ok_or(eyre::eyre!("Error reading tempdir path"))?;

    let shallow = fetch::Shallow::DepthAtRemote(1.try_into().expect("non-zero"));
    let url = gix::url::parse(url.into())?;
    let (checkout, _) = gix::prepare_clone(url, temp_path)
        .wrap_err_with(|| "Error cloning repo")?
        .with_shallow(shallow)
        .fetch_only(progress::Discard, &interrupt::IS_INTERRUPTED)
        .wrap_err_with(|| "Error fetching")?;

    // Gix supports shallow clones but git2 does not, so we have to use both libraries for now.
    // Currently gix does not have full support for checkouts (missing support for submodules) so we use git2 for this
    let repo = Repository::discover(checkout.path()).wrap_err_with(|| "Error discovering repo")?;
    repo.checkout_head(None)
        .wrap_err_with(|| "Error checking out HEAD")?;

    let mut stats = Language::new();
    let mut languages = Languages::new();
    languages.get_statistics(&[temp_path], &[], &Config::default());

    for (_, language) in languages {
        stats += language;
    }

    for stat in &mut stats.reports {
        stat.name = stat.name.strip_prefix(temp_path)?.to_owned();
    }

    Ok(cached::Return::new(stats))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt().with_ansi(false).init();
    vercel_runtime::run(handler).await
}
