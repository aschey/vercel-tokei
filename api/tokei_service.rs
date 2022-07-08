use badge::{Badge, BadgeOptions};
use cached::Cached;
use eyre::Context;
use git2::{Direction, Remote, Repository};
use log::info;
use std::error::Error;
use tempfile::TempDir;
use tokei::{Config, Language, Languages};
use vercel_lambda::{
    error::VercelError, http::StatusCode, lambda, IntoResponse, Request, Response,
};

const BILLION: usize = 1_000_000_000;
const BLANKS: &str = "blank lines";
const BLUE: &str = "#007ec6";
const CODE: &str = "lines of code";
const COMMENTS: &str = "comments";
const FILES: &str = "files";
const LINES: &str = "total lines";
const MILLION: usize = 1_000_000;
const THOUSAND: usize = 1_000;
const DAY_IN_SECONDS: u64 = 24 * 60 * 60;

fn handler(req: Request) -> Result<impl IntoResponse, VercelError> {
    let (parts, _) = req.into_parts();
    let paths: Vec<_> = parts.uri.path().split('/').collect();
    if paths.len() != 5 {
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(
                "path must have the folllowing structure: /tokei/{domain}/{user}/{repo}"
                    .to_string(),
            )
            .expect("Internal Server Error"));
    }
    let (domain, user, repo) = (paths[2], paths[3], paths[4]);
    let category = "lines";
    let mut domain = percent_encoding::percent_decode_str(domain)
        .decode_utf8()
        .map_err(|e| VercelError::new(&e.to_string()[..]))?;

    if !domain.contains('.') {
        domain += ".com";
    }

    let url = format!("https://{}/{}/{}", domain, user, repo);
    info!("Getting info for {url}");

    let mut repo = match Remote::create_detached(&url[..]) {
        Ok(repo) => repo,
        Err(e) => return Ok(bad_request(e.to_string())),
    };

    if let Err(e) = repo.connect(Direction::Fetch) {
        return Ok(bad_request(format!(
            "Error connecting to repository: {}",
            e
        )));
    }

    let repo_list = match repo.list() {
        Ok(list) => list,
        Err(e) => return Ok(bad_request(format!("Error listing repo contents: {}", e))),
    };

    let sha = match repo_list.first() {
        Some(sha) => sha.oid().to_string(),
        None => return Ok(bad_request("Repo contains no refs".to_string())),
    };
    info!("Repo sha: {sha:?}");

    if let Some(badge) = CACHE
        .lock()
        .unwrap()
        .cache_get(&repo_identifier(&url, &sha))
    {
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "image/svg+xml")
            .header("Cache-Control", "s-maxage=60, stale-while-revalidate=300")
            .body(make_badge(badge, category))
            .expect("Internal Server Error"));
    }

    let stats = get_statistics(&url, &sha)
        .map_err(|e| VercelError::new(&e.to_string()[..]))?
        .value;

    let badge = make_badge(&stats, category);

    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "image/svg+xml")
        .header("Cache-Control", "s-maxage=60, stale-while-revalidate=300")
        .body(badge)
        .expect("Internal Server Error");

    Ok(response)
}

fn bad_request(body: String) -> Response<String> {
    return Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(body)
        .expect("Internal Server Error");
}

fn repo_identifier(url: &str, sha: &str) -> String {
    format!("{}#{}", url, sha)
}

fn trim_and_float(num: usize, trim: usize) -> f64 {
    (num as f64) / (trim as f64)
}

fn make_badge(stats: &Language, category: &str) -> String {
    let (amount, label) = match &*category {
        "code" => (stats.code, CODE),
        "files" => (stats.reports.len(), FILES),
        "blanks" => (stats.blanks, BLANKS),
        "comments" => (stats.comments, COMMENTS),
        _ => (stats.code, LINES),
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

    let options = BadgeOptions {
        subject: String::from(label),
        status: amount,
        color: String::from(BLUE),
    };

    Badge::new(options).unwrap().to_svg()
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
    let temp_path = temp_dir.path().to_str().unwrap();

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
    Ok(lambda!(handler))
}
