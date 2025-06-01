use std::collections::HashMap;
use std::fs::DirEntry;
use std::time::{Duration, SystemTime};
use std::{fs, io, process};

use cached::Cached;
use eyre::Context;
use git2::{Direction, Remote, RemoteHead, Repository};
use gix::remote::fetch;
use gix::{interrupt, progress};
use http::{Method, StatusCode};
use rsbadges::Badge;
use tempfile::TempDir;
use tokei::{Config, Language, LanguageType, Languages};
use tracing::{error, info, warn};
use url::Url;
use vercel_runtime::{Body, Error, Request, Response};
use vercel_tokei::content_type::ContentType;
use vercel_tokei::settings::Settings;
use vercel_tokei::util::internal_server_error;

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

    let hash_query: HashMap<_, _> = parsed_url
        .query_pairs()
        .map(|p| (p.0.to_ascii_lowercase(), p.1))
        .collect();

    info!("Query pairs: {hash_query:?}");

    let settings = match Settings::from_query(&hash_query) {
        Ok(settings) => settings,
        Err(e) => return bad_request(e.to_string()),
    };
    let language_filter: Option<Vec<LanguageType>> = if let Some(languages) = &settings.languages {
        let languages: Result<Vec<LanguageType>, Result<Response<Body>, Error>> = languages
            .iter()
            .map(|l| {
                LanguageType::from_name(l).ok_or_else(|| {
                    bad_request(format!(
                        "Unknown language: {l}. Please note that languages are case sensitive and \
                         should be capitalized."
                    ))
                })
            })
            .collect();
        match languages {
            Ok(l) => Some(l),
            Err(e) => return e,
        }
    } else {
        None
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

    let url = format!("https://{domain}/{user}/{repo}");
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

    let sha = match find_sha(settings.branch.as_deref(), repo_list) {
        Ok(sha) => sha,
        Err(e) => return bad_request(e.to_string()),
    };
    info!("Repo sha: {sha:?} for branch {:?}", settings.branch);

    if let Some(badge) = CACHE
        .lock()
        .expect("Cache mutex poisoned")
        .cache_get(&cache_key(&url, &sha, &settings))
    {
        info!("Serving from cache");
        return match make_badge(&settings, badge) {
            Ok(badge) => build_response(badge, &settings),
            Err(e) => bad_request(e.to_string()),
        };
    }

    let stats = get_statistics(&url, &sha, &settings, language_filter)
        .wrap_err_with(|| "Error getting statistics")?
        .value;

    match make_badge(&settings, &stats).map_err(internal_server_error) {
        Ok(badge) => build_response(badge, &settings),
        Err(e) => bad_request(e.to_string()),
    }
}

fn find_sha(
    branch: Option<&str>,
    repo_list: &[RemoteHead],
) -> Result<String, Box<dyn std::error::Error>> {
    let head = match branch {
        Some(ref branch) => {
            let search = format!("refs/heads/{branch}");
            let found_branch = repo_list.iter().find(|r| r.name() == search.as_str());
            let Some(found_branch) = found_branch else {
                return Err(format!("Requested branch {branch:?} not found").into());
            };
            found_branch
        }
        None => {
            let Some(first) = repo_list.first() else {
                return Err("Repo contains no refs".into());
            };
            first
        }
    };
    Ok(head.oid().to_string())
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

fn cache_key(url: &str, sha: &str, settings: &Settings) -> String {
    format!("{}#{}#{}", url, sha, settings.loc_cache_key())
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
    ty = "cached::TimedSizedCache<String, cached::Return<Language>>",
    create = "{ cached::TimedSizedCache::with_size_and_lifespan(1000, DAY_IN_SECONDS) }",
    convert = r#"{ cache_key(url, _sha, settings) }"#
)]
fn get_statistics(
    url: &str,
    _sha: &str,
    settings: &Settings,
    language_filter: Option<Vec<LanguageType>>,
) -> eyre::Result<cached::Return<Language>> {
    let temp_prefix = "tokei-cache";
    let _ = clear_previous_files(temp_prefix).inspect_err(|e| warn!("error cleaning files: {e:?}"));

    let temp_dir = match TempDir::with_prefix(temp_prefix) {
        Ok(temp_dir) => temp_dir,
        Err(e) => {
            error!("Failed to create temp dir: {e:?}. Force exiting process");
            // If we failed to create the temp dir, the disk is likely full.
            // Force exit the process to force a new container to be created.
            process::exit(1);
        }
    };
    let temp_path = temp_dir
        .path()
        .to_str()
        .ok_or(eyre::eyre!("Error reading tempdir path"))?;

    let shallow = fetch::Shallow::DepthAtRemote(1.try_into().expect("non-zero"));
    let url = gix::url::parse(url.into())?;
    let (checkout, _) = gix::prepare_clone(url, temp_path)
        .wrap_err_with(|| "Error cloning repo")?
        .with_shallow(shallow)
        .with_ref_name(settings.branch.as_deref())?
        .fetch_only(progress::Discard, &interrupt::IS_INTERRUPTED)
        .wrap_err_with(|| "Error fetching")?;

    // Gix supports shallow clones but git2 does not, so we have to use both libraries for now.
    // Currently gix does not have full support for checkouts (missing support for submodules) so we
    // use git2 for this
    let repo = Repository::discover(checkout.path()).wrap_err_with(|| "Error discovering repo")?;
    repo.checkout_head(None)
        .wrap_err_with(|| "Error checking out HEAD")?;

    let mut stats = Language::new();
    let mut languages = Languages::new();
    let config = Config {
        types: language_filter,
        ..Default::default()
    };

    languages.get_statistics(&[temp_path], &[], &config);

    for (_, language) in languages {
        stats += language;
    }

    for stat in &mut stats.reports {
        stat.name = stat.name.strip_prefix(temp_path)?.to_owned();
    }
    let _ = temp_dir
        .close()
        .map_err(|e| warn!("error removing temporary directory: {e:?}"));

    Ok(cached::Return::new(stats))
}

fn clear_previous_files(prefix: &str) -> io::Result<()> {
    // Previous temp files should get removed automatically, but we'll force remove any old files
    // here just in case.
    let env_temp = std::env::temp_dir();
    let cache_prefix = env_temp.join(prefix);
    let cache_prefix = cache_prefix.as_os_str().as_encoded_bytes();
    for entry in fs::read_dir(env_temp)? {
        let _ =
            clear_entry(entry, cache_prefix).inspect_err(|e| warn!("failed to clean entry: {e:?}"));
    }

    Ok(())
}

fn clear_entry(entry: io::Result<DirEntry>, cache_prefix: &[u8]) -> io::Result<()> {
    let entry = entry?;
    let path = entry.path();

    if path.is_dir()
        && path
            .as_os_str()
            .as_encoded_bytes()
            .starts_with(cache_prefix)
    {
        let created = path.metadata()?.created()?;
        if created < SystemTime::now() - Duration::from_secs(60) {
            info!("removing old path: {path:?}");
            fs::remove_dir_all(path).unwrap();
        }
    }

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt().with_ansi(false).init();
    vercel_runtime::run(handler).await
}
