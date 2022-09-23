use cached::Cached;
use eyre::Context;
use git2::{Direction, Remote, Repository};
use log::{error, info};
use rsbadges::Badge;
use std::{collections::HashMap, error::Error};
use tempfile::TempDir;
use tokei::{Config, Language, Languages};
use url::Url;
use vercel_lambda::{
    error::VercelError,
    http::{Method, StatusCode},
    lambda, IntoResponse, Request, Response,
};

use crate::{category::Category, content_type::ContentType, settings::Settings};

mod category;
mod color;
mod content_type;
mod settings;
mod style;
mod theme;

const BILLION: usize = 1_000_000_000;
const MILLION: usize = 1_000_000;
const THOUSAND: usize = 1_000;
const DAY_IN_SECONDS: u64 = 24 * 60 * 60;
const REVALIDATE_FACTOR: u32 = 5;

fn handler(req: Request) -> Result<impl IntoResponse, VercelError> {
    // For health checks
    if req.method() == Method::HEAD {
        return Ok(Response::new("".to_string()));
    }

    let url = Url::parse(&req.uri().to_string().replace('#', "%23"))
        .map_err(|e| internal_server_error(Box::new(e)))?;

    let paths = url.path_segments().unwrap().collect::<Vec<_>>();
    if paths.len() != 4 {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(
                "path must have the folllowing structure: /tokei/{domain}/{user}/{repo}"
                    .to_string(),
            )
            .map_err(|e| internal_server_error(Box::new(e)));
    }

    let pairs = url.query_pairs().collect::<HashMap<_, _>>();
    let settings = match Settings::from_query(&pairs) {
        Ok(settings) => settings,
        Err(e) => return bad_request(e.to_string()),
    };

    let (domain, user, repo) = (paths[1], paths[2], paths[3]);
    let mut domain = percent_encoding::percent_decode_str(domain)
        .decode_utf8()
        .map_err(|e| VercelError::new(e.to_string().as_ref()))?;

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
        .unwrap()
        .cache_get(&repo_identifier(&url, &sha))
    {
        info!("Serving from cache");
        return match make_badge(&settings, badge) {
            Ok(badge) => build_response(badge, &settings),
            Err(e) => bad_request(e.to_string()),
        };
    }

    let stats = get_statistics(&url, &sha)
        .map_err(|e| VercelError::new(&e.to_string()[..]))?
        .value;

    match make_badge(&settings, &stats).map_err(internal_server_error) {
        Ok(badge) => build_response(badge, &settings),
        Err(e) => bad_request(e.to_string()),
    }
}

fn internal_server_error(err: Box<dyn Error>) -> VercelError {
    error!("{err:?}");
    VercelError::new("Internal Server Error")
}

fn build_response(badge: String, settings: &Settings) -> Result<Response<String>, VercelError> {
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
        .body(badge)
        .map_err(|e| internal_server_error(Box::new(e)))
}

fn bad_request(body: String) -> Result<Response<String>, VercelError> {
    return Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(body)
        .map_err(|e| internal_server_error(Box::new(e)));
}

fn repo_identifier(url: &str, sha: &str) -> String {
    format!("{}#{}", url, sha)
}

fn trim_and_float(num: usize, trim: usize) -> f64 {
    (num as f64) / (trim as f64)
}

fn make_badge(settings: &Settings, stats: &Language) -> Result<String, Box<dyn Error>> {
    if settings.content_type == ContentType::Json {
        return Ok(serde_json::to_string(&stats)?);
    }

    let amount = settings.category.stats(stats);
    let label = settings.category.description();

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
        label_text: String::from(label),
        label_color: settings.theme.label_color.to_string(),
        msg_text: amount,
        msg_color: settings.theme.color.to_string(),
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

    Repository::clone(url, temp_path).wrap_err_with(|| "Error cloning repo")?;

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

#[allow(dead_code)]
fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    #[allow(clippy::unit_arg)]
    Ok(lambda!(handler))
}
