use clap::Parser;
use color_eyre::{Report, Result};
use colored::{Color, Colorize};
use std::{fs::File, io::BufReader, path::PathBuf};
use tracing::info;
use tree_sitter_highlight::{Highlight, HighlightConfiguration, HighlightEvent, Highlighter};
use wasm2spirv::{config::Config, Compilation};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// File to be converted. Has to be a WebAssembly text or binary file
    source: PathBuf,

    /// Import compilation configuration from a custom section on the WebAssemly program itself
    #[arg(long, default_value_t = false)]
    from_wasm: bool,

    /// Import compilation configuration from JSON file
    #[arg(long)]
    from_json: Option<PathBuf>,

    /// Path to write the compiled spv file
    #[arg(long, short)]
    output: Option<PathBuf>,

    /// Disables logging
    #[arg(long, short, default_value_t = false)]
    quiet: bool,

    /// When printing to the standard output, syntax highlights will be added.
    /// (Currently, it only works for assembly and GLSL outputs)
    #[arg(long)]
    highlight: bool,

    /// Optimizes the compiled result
    #[cfg(feature = "spirv-tools")]
    #[arg(long, default_value_t = false)]
    optimize: bool,

    /// Validates the resulting SPIR-V
    #[cfg(any(feature = "naga-validate", feature = "spvt-validate"))]
    #[arg(long, default_value_t = false)]
    validate: bool,

    /// Print OpenGL Shading Language (GLSL) translation to standard output
    #[cfg(any(feature = "spvc-glsl", feature = "naga-glsl"))]
    #[arg(long, default_value_t = false)]
    show_glsl: bool,

    /// Print High Level Shading Language (HLSL) translation to standard output
    #[cfg(any(feature = "spvc-hlsl", feature = "naga-hlsl"))]
    #[arg(long, default_value_t = false)]
    show_hlsl: bool,

    /// Print Metal Shading Language (MSL) translation to standard output
    #[cfg(any(feature = "spvc-msl", feature = "naga-msl"))]
    #[arg(long, default_value_t = false)]
    show_msl: bool,

    /// Print WebGPU Shading Language (WGSL) translation to standard output
    #[cfg(feature = "naga-wgsl")]
    #[arg(long, default_value_t = false)]
    show_wgsl: bool,

    /// Print text assembly on standard output
    #[arg(long, default_value_t = false)]
    show_asm: bool,
}

pub fn main() -> color_eyre::Result<()> {
    let _ = color_eyre::install();

    let Cli {
        source,
        from_wasm,
        from_json,
        output,
        quiet,
        highlight,
        #[cfg(feature = "spirv-tools")]
        optimize,
        #[cfg(any(feature = "naga-validate", feature = "spvt-validate"))]
        validate,
        show_asm,
        #[cfg(any(feature = "spvc-glsl", feature = "naga-glsl"))]
        show_glsl,
        #[cfg(any(feature = "spvc-hlsl", feature = "naga-hlsl"))]
        show_hlsl,
        #[cfg(any(feature = "spvc-msl", feature = "naga-msl"))]
        show_msl,
        #[cfg(feature = "naga-wgsl")]
        show_wgsl,
    } = Cli::parse();

    if !quiet {
        tracing_subscriber::fmt::try_init().map_err(Report::msg)?;
    }

    let mut config: Config = match (from_wasm, from_json) {
        (true, None) => todo!(),
        (false, Some(json)) => {
            let mut file = BufReader::new(File::open(json)?);
            serde_json::from_reader(&mut file)?
        }
        (false, None) => {
            return Err(Report::msg(
                "One of 'from-wasm', 'from-binary' or 'from-json' must be enabled",
            ));
        }
        _ => {
            return Err(Report::msg(
                "Only one of 'from-wasm', 'from-binary' or 'from-json' must be enabled",
            ))
        }
    };
    config.enable_capabilities()?;

    let bytes = wat::parse_file(source)?;
    let mut compilation = Compilation::new(config, &bytes)?;

    #[cfg(any(feature = "naga-validate", feature = "spvt-validate"))]
    if validate {
        compilation.validate()?;
    }

    #[cfg(feature = "spirv-tools")]
    if optimize {
        compilation = compilation.into_optimized()?;
    }

    if let Some(output) = output {
        let bytes = compilation.bytes()?;
        std::fs::write(output, &bytes)?;
    }

    if show_asm {
        use tree_sitter_asm::HIGHLIGHTS_QUERY;
        print_to_stdout(
            tree_sitter_asm::language,
            HIGHLIGHTS_QUERY,
            highlight,
            compilation.assembly()?,
        )?;
    }

    #[cfg(any(feature = "spvc-glsl", feature = "naga-glsl"))]
    if show_glsl {
        use tree_sitter_glsl::HIGHLIGHTS_QUERY;
        print_to_stdout(
            tree_sitter_glsl::language,
            HIGHLIGHTS_QUERY,
            highlight,
            compilation.glsl()?,
        )?;
    }

    #[cfg(any(feature = "spvc-hlsl", feature = "naga-hlsl"))]
    if show_hlsl {
        print_to_stdout(
            tree_sitter_hlsl::language,
            include_str!("../queries/hlsl-highlights.scm"),
            highlight,
            compilation.hlsl()?,
        )?;
    }

    #[cfg(any(feature = "spvc-msl", feature = "naga-msl"))]
    if show_msl {
        use tree_sitter_c::HIGHLIGHT_QUERY;
        print_to_stdout(
            tree_sitter_c::language,
            HIGHLIGHT_QUERY,
            highlight,
            compilation.msl()?,
        )?;
    }

    #[cfg(feature = "naga-wgsl")]
    if show_wgsl {
        println!("{}", compilation.wgsl()?);
    }

    return Ok(());
}

fn print_to_stdout(
    language: impl FnOnce() -> tree_sitter::Language,
    highlights_query: &'static str,
    highlight: bool,
    s: &str,
) -> color_eyre::Result<()> {
    const AQUA: Color = Color::TrueColor {
        r: 5,
        g: 195,
        b: 221,
    };

    if !highlight {
        print!("{}", s);
        return Ok(());
    }

    macro_rules! highlights {
        ($($name:literal => $color:expr),+ $(,)?) => {
            const HIGHLIGHT_NAMES: &'static [&'static str] = &[
                $($name),+
            ];

            const HIGHLIGHT_COLORS: &'static [Color] = &[
                $($color),+
            ];
        };
    }

    highlights! {
        "number" => Color::TrueColor {
            r: 200,
            g: 220,
            b: 143
        },
        "attribute" => Color::Blue,
        "constant" => Color::Red,
        "function" => Color::BrightYellow,
        "keyword" => Color::Magenta,
        "property" => Color::TrueColor {
            r: 170,
            g: 219,
            b: 30
        },
        "punctuation.bracket" => Color::Magenta,
        "string" => Color::TrueColor {
            r: 160,
            g: 82,
            b: 45
        },
        "tag" => Color::Red,
        "type" => Color::Green,
        "variable" => AQUA,
    }

    let mut config = HighlightConfiguration::new(language(), highlights_query, "", "")?;
    config.configure(HIGHLIGHT_NAMES);

    let mut highlighter = Highlighter::new();
    let mut highlights = highlighter.highlight(&config, s.as_bytes(), None, |_| None)?;

    let mut ended_in_new_line = false;
    loop {
        let (start, end, highlight);

        match highlights.next().transpose()? {
            Some(HighlightEvent::HighlightStart(Highlight(x))) => {
                highlight = Some(x);
                (start, end) = match highlights.next().transpose()? {
                    Some(HighlightEvent::Source { start, end }) => (start, end),
                    _ => continue,
                };
            }
            Some(HighlightEvent::Source { start: s, end: e }) => {
                highlight = None;
                (start, end) = (s, e);
            }
            Some(HighlightEvent::HighlightEnd) => continue,
            None => break,
        }

        let entry = &s[start..end];
        ended_in_new_line = entry.chars().last().is_some_and(|x| x == '\n');

        if let Some(color_idx) = highlight {
            print!("{}", entry.color(HIGHLIGHT_COLORS[color_idx]))
        } else {
            print!("{entry}");
        }
    }

    if !ended_in_new_line {
        println!("");
    }

    return Ok(());
}
