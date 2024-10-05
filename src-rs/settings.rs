use std::borrow::Cow;
use std::collections::HashMap;

use crate::category::Category;
use crate::content_type::ContentType;
use crate::theme::Theme;

const DEFAULT_CACHE_SECONDS: u32 = 60;

pub struct Settings {
    pub category: Category,
    pub content_type: ContentType,
    pub theme: Theme,
    pub cache_seconds: u32,
    pub label: Option<String>,
    pub logo: Option<String>,
    pub logo_as_label: bool,
    pub branch: Option<String>,
}

impl Settings {
    pub fn from_query(query: &HashMap<Cow<str>, Cow<str>>) -> Result<Self, &'static str> {
        let category = Category::from_query(query)?;
        let content_type = ContentType::from_query(query)?;
        let theme = Theme::from_query(query)?;

        let label = query.get("label").map(|label| label.to_string());
        let logo = query.get("logo").map(|label| label.to_string());
        let logo_as_label = query
            .get("logoAsLabel")
            .map(|l| l == "1" || l.to_lowercase() == "true")
            .unwrap_or(false);

        let mut cache_seconds: u32 = match query.get("cacheSeconds") {
            Some(seconds) => seconds.parse().unwrap_or(DEFAULT_CACHE_SECONDS),
            None => DEFAULT_CACHE_SECONDS,
        };
        if cache_seconds < DEFAULT_CACHE_SECONDS {
            cache_seconds = DEFAULT_CACHE_SECONDS;
        }
        let branch = query.get("branch").map(|branch| branch.to_string());

        Ok(Self {
            cache_seconds,
            category,
            theme,
            content_type,
            label,
            logo,
            logo_as_label,
            branch,
        })
    }
}
