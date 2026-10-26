use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "pullpen", version, about)]
pub struct Args {
    /// Config file to read instead of $XDG_CONFIG_HOME/pullpen/config.toml.
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Run offline on the design's fixture data, against a fake forge.
    #[arg(long)]
    pub fixtures: bool,

    /// Glyph tier. Swaps characters only; widths never change between tiers.
    #[arg(long, value_enum)]
    pub glyphs: Option<Glyphs>,

    /// Colour tier. Detected from COLORTERM and TERM when omitted.
    #[arg(long, value_enum)]
    pub colors: Option<Colors>,

    /// Section to open on, by name or short name.
    #[arg(long, value_name = "NAME")]
    pub section: Option<String>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Glyphs {
    Unicode,
    Ascii,
    Nerd,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Colors {
    #[value(name = "16")]
    C16,
    #[value(name = "256")]
    C256,
    #[value(name = "true")]
    True,
}
