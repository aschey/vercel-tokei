use std::borrow::Cow;
use std::collections::HashMap;

use crate::color::Color;
use crate::style::Style;

pub struct Theme {
    pub style: Style,
    pub label_color: Color,
    pub color: Color,
}

impl Theme {
    pub fn from_query(query: &HashMap<String, Cow<str>>) -> Result<Self, &'static str> {
        let style = Style::from_query(query)?;
        let label_color = Color::from_query(query, "labelColor", Color::Grey);
        let color = Color::from_query(query, "color", Color::Blue);
        Ok(Self {
            style,
            label_color,
            color,
        })
    }
}
