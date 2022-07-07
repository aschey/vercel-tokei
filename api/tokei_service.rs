use badge::{Badge, BadgeOptions};
use cached::Cached;
use git2::{build::RepoBuilder, Direction, Remote, Repository};
use std::{error::Error, process::Command};
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
const HASH_LENGTH: usize = 40;
const LINES: &str = "total lines";
const MILLION: usize = 1_000_000;
const THOUSAND: usize = 1_000;
const DAY_IN_SECONDS: u64 = 24 * 60 * 60;

fn handler(req: Request) -> Result<impl IntoResponse, VercelError> {
    let (parts, body) = req.into_parts();
    let paths: Vec<_> = parts.uri.path().split('/').collect();
    let (domain, user, repo) = ("github.com", "aschey", "platune"); //(paths[0], paths[1], paths[2]);
    let category = parts
        .uri
        .query()
        .map(|q| q.split('=').collect::<Vec<_>>().get(1).unwrap().to_string())
        .unwrap_or_else(|| String::from("lines"));
    let mut domain = percent_encoding::percent_decode_str(domain)
        .decode_utf8()
        .unwrap();
    // For backwards compatibility if a domain isn't specified we append `.com`.
    if !domain.contains('.') {
        domain += ".com";
    }
    let url = format!("https://{}/{}/{}", domain, user, repo);

    let mut repo = Remote::create_detached("https://github.com/aschey/platune").unwrap();
    repo.connect(Direction::Fetch).unwrap();
    let sha = repo.list().unwrap().first().unwrap().oid().to_string();
    println!("{}", sha);
    if CACHE
        .lock()
        .unwrap()
        .cache_get(&repo_identifier(&url, &sha))
        .is_some()
    {
        return Ok(Response::builder().status(304).body("".to_owned()).unwrap());
    }

    let entry = get_statistics(&url, &sha).unwrap();
    let stats = entry.value;

    let badge = make_badge(&stats, &category);

    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "image/svg+xml")
        .body(badge)
        .expect("Internal Server Error");

    Ok(response)
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

    Repository::clone(url, temp_path).unwrap();

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
    Ok(lambda!(handler))
}
