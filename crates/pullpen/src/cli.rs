use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

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

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Sign in to your forges and answer what pullpen needs to know.
    Setup,
    /// See and change every setting; `get` and `set` do it from a script.
    Settings {
        #[command(subcommand)]
        action: Option<SettingsAction>,
    },
    /// Check config.toml and each forge's sign-in, and say how to fix what fails.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    Check,
}

#[derive(Subcommand, Debug)]
pub enum SettingsAction {
    /// Print one setting's value.
    Get { key: String },
    /// Change one setting.
    Set {
        key: String,
        #[arg(num_args = 0.., trailing_var_arg = true)]
        value: Vec<String>,
    },
    /// Where the config and credentials files are.
    Path,
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
