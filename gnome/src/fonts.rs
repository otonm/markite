//! Which font families are installed, so the preferences only offer fonts that exist.

use gtk::prelude::*;
use std::collections::HashSet;

pub struct Fonts {
    families: HashSet<String>,
}

impl Fonts {
    /// Reads the families known to Pango (fontconfig) for the widget's display.
    pub fn detect(widget: &impl IsA<gtk::Widget>) -> Fonts {
        let families: HashSet<String> = widget
            .pango_context()
            .list_families()
            .iter()
            .map(|f| f.name().to_string())
            .collect();
        markite_core::trace!("fonts: {} families installed", families.len());
        Fonts { families }
    }

    pub fn has(&self, family: &str) -> bool {
        self.families.contains(family)
    }

    /// The installed ones among `wanted`, in the given order.
    pub fn available(&self, wanted: &[&str]) -> Vec<String> {
        wanted
            .iter()
            .filter(|f| self.has(f))
            .map(|f| f.to_string())
            .collect()
    }
}
