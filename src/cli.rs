use clap::Parser;
use color_eyre::{Report, Result};
use std::{fs::File, io::BufReader, path::PathBuf};
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};
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
        println!("{}", compilation.assembly()?)
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
        println!("{}", compilation.hlsl()?);
    }

    #[cfg(any(feature = "spvc-msl", feature = "naga-msl"))]
    if show_msl {
        use tree_sitter_cpp::HIGHLIGHT_QUERY;
        print_to_stdout(
            tree_sitter_cpp::language,
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
    if !highlight {
        print!("{}", s);
        return Ok(());
    }

    let highlight_names = &[
        "attribute",
        "constant",
        "function.builtin",
        "function",
        "keyword",
        "operator",
        "property",
        "punctuation",
        "punctuation.bracket",
        "punctuation.delimiter",
        "string",
        "string.special",
        "tag",
        "type",
        "type.builtin",
        "variable",
        "variable.builtin",
        "variable.parameter",
    ];

    let mut config = HighlightConfiguration::new(language(), highlights_query, "", "")?;
    config.configure(highlight_names);

    let mut highlighter = Highlighter::new();
    let mut highlights = highlighter.highlight(&config, s.as_bytes(), None, |_| None)?;

    loop {
        match highlights.next().transpose()? {
            Some(HighlightEvent::Source { start, end }) => {
                let entry = &s[start..end];
                let highlight = match highlights.next().transpose()? {
                    Some(HighlightEvent::HighlightStart(x)) => x,
                    _ => break,
                };
                println!("{entry}");
                let _ = highlights.next();
            }
            _ => break,
        }
    }

    return Ok(());
}
