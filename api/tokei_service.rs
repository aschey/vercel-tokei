use badge::{Badge, BadgeOptions};
use cached::Cached;
use eyre::Context;
use git2::{Direction, Remote, Repository};
use log::{error, info};
use std::{borrow::Cow, collections::HashMap, error::Error};
use tempfile::TempDir;
use tokei::{Config, Language, Languages};
use url::Url;
use vercel_lambda::{
    error::VercelError,
    http::{Method, StatusCode},
    lambda, IntoResponse, Request, Response,
};

#[derive(PartialEq, Eq, Debug)]
enum ContentType {
    Svg,
    Json,
}

impl ContentType {
    fn from_query(query: &HashMap<Cow<str>, Cow<str>>) -> Result<Self, &'static str> {
        match query.get("format") {
            Some(format) => match format.to_lowercase().as_str() {
                "svg" => Ok(Self::Svg),
                "json" => Ok(Self::Json),
                _ => Err("Invalid format parameter. Choices are 'svg' and 'json'"),
            },
            None => Ok(Self::Svg),
        }
    }
}

impl ToString for ContentType {
    fn to_string(&self) -> String {
        match self {
            Self::Svg => "image/svg+xml".to_owned(),
            Self::Json => "application/json".to_owned(),
        }
    }
}

enum Category {
    Blanks,
    Code,
    Comments,
    Files,
}

impl Category {
    fn description(&self) -> &str {
        match self {
            Category::Blanks => "blank lines",
            Category::Code => "lines of code",
            Category::Comments => "comments",
            Category::Files => "files",
        }
    }

    fn from_query(query: &HashMap<Cow<str>, Cow<str>>) -> Result<Self, &'static str> {
        match query.get("category") {
            Some(category) => match category.to_lowercase().as_str() {
                "code" => Ok(Self::Code),
                "files" => Ok(Self::Files),
                "blanks" => Ok(Self::Blanks),
                "comments" => Ok(Self::Comments),
                _ => Err(
                    "Invalid category parameter. Choices are 'code', 'files', 'blanks', and 'comments'",
                ),
            },
            None => Ok(Self::Code),
        }
    }

    fn stats(&self, language: &Language) -> usize {
        match self {
            Self::Blanks => language.blanks,
            Self::Files => language.reports.len(),
            Self::Comments => language.comments,
            Self::Code => language.code,
        }
    }
}

const BILLION: usize = 1_000_000_000;
const BLUE: &str = "#007ec6";
const MILLION: usize = 1_000_000;
const THOUSAND: usize = 1_000;
const DAY_IN_SECONDS: u64 = 24 * 60 * 60;

fn handler(req: Request) -> Result<impl IntoResponse, VercelError> {
    // For health checks
    if req.method() == Method::HEAD {
        return Ok(Response::new("".to_string()));
    }

    let url = Url::parse(&req.uri().to_string()).map_err(|e| internal_server_error(Box::new(e)))?;
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
    let content_type = match ContentType::from_query(&pairs) {
        Ok(content_type) => content_type,
        Err(e) => return bad_request(e.to_string()),
    };

    let (domain, user, repo) = (paths[1], paths[2], paths[3]);
    let category = match Category::from_query(&pairs) {
        Ok(category) => category,
        Err(e) => return bad_request(e.to_string()),
    };

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
        let badge = make_badge(&content_type, badge, &category)
            .map_err(|e| VercelError::new(&e.to_string()))?;
        return build_response(badge, &content_type);
    }

    let stats = get_statistics(&url, &sha)
        .map_err(|e| VercelError::new(&e.to_string()[..]))?
        .value;

    let badge = make_badge(&content_type, &stats, &category).map_err(internal_server_error)?;

    build_response(badge, &content_type)
}

fn internal_server_error(err: Box<dyn Error>) -> VercelError {
    error!("{err:?}");
    VercelError::new("Internal Server Error")
}

fn build_response(
    badge: String,
    content_type: &ContentType,
) -> Result<Response<String>, VercelError> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", content_type.to_string())
        .header("Cache-Control", "s-maxage=60, stale-while-revalidate=300")
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

fn make_badge(
    content_type: &ContentType,
    stats: &Language,
    category: &Category,
) -> Result<String, Box<dyn Error>> {
    if *content_type == ContentType::Json {
        return Ok(serde_json::to_string(&stats)?);
    }

    let amount = category.stats(stats);
    let label = category.description();

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

    Ok(Badge::new(options)?.to_svg())
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
