use std::{borrow::Cow, collections::HashMap};

use crate::{content_type::ContentType, theme::Theme, Category};

const DEFAULT_CACHE_SECONDS: u32 = 60;

pub(crate) struct Settings {
    pub(crate) category: Category,
    pub(crate) content_type: ContentType,
    pub(crate) theme: Theme,
    pub(crate) cache_seconds: u32,
    pub(crate) label: Option<String>,
    pub(crate) logo: Option<String>,
    pub(crate) logo_as_label: bool,
}

impl Settings {
    pub(crate) fn from_query(query: &HashMap<Cow<str>, Cow<str>>) -> Result<Self, &'static str> {
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

        Ok(Self {
            cache_seconds,
            category,
            theme,
            content_type,
            label,
            logo,
            logo_as_label,
        })
    }
}
