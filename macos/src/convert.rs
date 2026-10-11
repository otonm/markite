//! Records, enums and free functions that only convert types between core and Swift.
use crate::errors::{check_path, MarkiteError};
use markite_core::{blocks, document, minimap, options};
use std::path::Path;

pub fn u32_sat(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

#[derive(uniffi::Record)]
pub struct Block {
    pub start_line: u32,
    pub end_line: u32,
    pub html: String,
}

#[derive(uniffi::Record)]
pub struct BlockPosition {
    pub index: u32,
    pub fraction: f64,
}

fn core_blocks(blocks: Vec<Block>) -> Vec<blocks::Block> {
    blocks
        .into_iter()
        .map(|b| blocks::Block {
            start_line: b.start_line as usize,
            end_line: b.end_line as usize,
            html: b.html,
        })
        .collect()
}

#[uniffi::export]
pub fn line_to_block(blocks: Vec<Block>, line: f64) -> BlockPosition {
    let (i, fraction) = blocks::line_to_block(&core_blocks(blocks), line);
    BlockPosition {
        index: u32_sat(i),
        fraction,
    }
}

#[uniffi::export]
pub fn block_to_line(blocks: Vec<Block>, index: u32, fraction: f64) -> f64 {
    blocks::block_to_line(&core_blocks(blocks), index as usize, fraction)
}

#[derive(uniffi::Enum)]
pub enum MinimapKind {
    Blank,
    Heading,
    Code,
    Text,
    ListItem,
    Quote,
    Table,
}

#[derive(uniffi::Record)]
pub struct MinimapLine {
    pub indent: u16,
    pub len: u16,
    pub kind: MinimapKind,
    pub marker: u16,
}

#[uniffi::export]
pub fn minimap_lines(text: String) -> Vec<MinimapLine> {
    use minimap::LineKind as K;
    minimap::lines(&text)
        .into_iter()
        .map(|l| MinimapLine {
            indent: l.indent,
            len: l.len,
            marker: l.marker,
            kind: match l.kind {
                K::Blank => MinimapKind::Blank,
                K::Heading => MinimapKind::Heading,
                K::Code => MinimapKind::Code,
                K::Text => MinimapKind::Text,
                K::ListItem => MinimapKind::ListItem,
                K::Quote => MinimapKind::Quote,
                K::Table => MinimapKind::Table,
            },
        })
        .collect()
}

#[uniffi::export]
pub fn read_limited(path: String) -> Result<Vec<u8>, MarkiteError> {
    Ok(document::read_limited(Path::new(check_path(&path)?))?)
}

#[uniffi::export]
pub fn max_file_bytes() -> u64 {
    document::MAX_FILE_BYTES
}

#[uniffi::export]
pub fn clamp_wrap_column(v: i32) -> i32 {
    options::clamp_wrap_column(v)
}

#[uniffi::export]
pub fn clamp_preview_width(v: i32) -> i32 {
    options::clamp_preview_width(v)
}

#[uniffi::export]
pub fn font_size(saved: i32) -> Option<i32> {
    options::font_size(saved)
}

#[uniffi::export]
pub fn theme_name(family: String, dark: bool) -> String {
    options::theme_name(&family, dark).to_owned()
}

#[derive(uniffi::Enum)]
pub enum AppearanceKind {
    System,
    Light,
    Dark,
}

impl From<options::Appearance> for AppearanceKind {
    fn from(a: options::Appearance) -> Self {
        match a {
            options::Appearance::System => Self::System,
            options::Appearance::Light => Self::Light,
            options::Appearance::Dark => Self::Dark,
        }
    }
}

impl From<AppearanceKind> for options::Appearance {
    fn from(a: AppearanceKind) -> Self {
        match a {
            AppearanceKind::System => Self::System,
            AppearanceKind::Light => Self::Light,
            AppearanceKind::Dark => Self::Dark,
        }
    }
}

#[uniffi::export]
pub fn appearance_parse(s: String) -> AppearanceKind {
    options::Appearance::parse(&s).into()
}

#[uniffi::export]
pub fn appearance_name(kind: AppearanceKind) -> String {
    options::Appearance::from(kind).as_str().to_owned()
}

fn strings(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

#[uniffi::export]
pub fn theme_families() -> Vec<String> {
    strings(&options::THEME_FAMILIES)
}

#[uniffi::export]
pub fn code_fonts() -> Vec<String> {
    strings(&options::CODE_FONTS)
}

#[uniffi::export]
pub fn sans_fonts() -> Vec<String> {
    strings(&options::SANS_FONTS)
}

#[uniffi::export]
pub fn serif_fonts() -> Vec<String> {
    strings(&options::SERIF_FONTS)
}

#[uniffi::export]
pub fn default_wrap_column() -> i32 {
    options::DEFAULT_WRAP_COLUMN
}

#[uniffi::export]
pub fn default_preview_width() -> i32 {
    options::DEFAULT_PREVIEW_WIDTH
}
