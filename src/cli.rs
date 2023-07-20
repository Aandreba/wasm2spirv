use clap::Parser;
use color_eyre::Report;
use rspirv::binary::{Assemble, Disassemble};
use std::{fs::File, io::BufReader, mem::ManuallyDrop, path::PathBuf};
use wasm2spirv_lib::{
    binary::deserialize::BinaryDeserialize, config::Config, fg::module::ModuleBuilder,
};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// File to be converted. Has to be a WebAssembly text or binary file
    source: PathBuf,

    /// Import compilation configuration from a custom section on the WebAssemly program itself
    #[arg(long, default_value_t = false)]
    from_wasm: bool,

    /// Import compilation configuration from binary file
    #[arg(long)]
    from_binary: Option<PathBuf>,

    /// Import compilation configuration from JSON file
    #[arg(long)]
    from_json: Option<PathBuf>,

    /// Path to write the compiled spv file
    #[arg(long, short)]
    output: Option<PathBuf>,

    /// Print text assembly on standard output (defaults to false)
    #[arg(long, default_value_t = false)]
    show_asm: bool,
}

pub fn main() -> color_eyre::Result<()> {
    let _ = color_eyre::install();
    tracing_subscriber::fmt::try_init().map_err(Report::msg)?;

    let Cli {
        source,
        from_wasm,
        from_binary,
        from_json,
        output,
        show_asm,
    } = Cli::parse();

    let config = match (from_wasm, from_binary, from_json) {
        (true, None, None) => todo!(),
        (false, Some(bin), None) => {
            let mut file = BufReader::new(File::open(bin)?);
            Config::deserialize_from(&mut file)?
        }
        (false, None, Some(json)) => {
            let mut file = BufReader::new(File::open(json)?);
            serde_json::from_reader(&mut file)?
        }
        (false, None, None) => {
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

    let bytes = wat::parse_file(source)?;
    let builder = ModuleBuilder::new(config, &bytes)?;
    let module = builder.translate()?.module();

    if show_asm {
        println!("{}", module.disassemble())
    }

    if let Some(output) = output {
        let mut words = ManuallyDrop::new(module.assemble());
        let bytes = unsafe {
            Vec::from_raw_parts(
                words.as_mut_ptr().cast::<u8>(),
                4 * words.len(),
                4 * words.capacity(),
            )
        };
        std::fs::write(output, &bytes)?;
    }

    return Ok(());
}

/*
#[test]
fn binary_config() -> color_eyre::Result<()> {
    let mut config = Config::builder(
        TargetPlatform::VK_1_1,
        None,
        CapabilityModel::default(),
        ExtensionModel::dynamic(vec![
            "SPV_KHR_variable_pointers",
            "SPV_KHR_storage_buffer_storage_class",
        ]),
        AddressingModel::Logical,
        MemoryModel::GLSL450,
    )?;
    config.set_features(WasmFeatures {
        memory64: true,
        ..Default::default()
    });

    let mut saxpy = config
        .function(2)
        .set_entry_point(ExecutionModel::GLCompute)?
        .set_exec_mode(ExecutionMode::LocalSize(1, 1, 1))?;

    saxpy = saxpy
        .param(0)
        .set_type(CompositeType::structured(ScalarType::I32))?
        .set_kind(ParameterKind::DescriptorSet {
            storage_class: StorageClass::StorageBuffer,
            set: 0,
            binding: 0,
        })?
        .build();

    saxpy = saxpy
        .param(1)
        .set_type(CompositeType::structured(ScalarType::F32))?
        .set_kind(ParameterKind::DescriptorSet {
            storage_class: StorageClass::StorageBuffer,
            set: 0,
            binding: 1,
        })?
        .build();

    saxpy = saxpy
        .param(2)
        .set_extern_pointer(true)
        .set_kind(ParameterKind::DescriptorSet {
            storage_class: StorageClass::StorageBuffer,
            set: 0,
            binding: 2,
        })?
        .set_type(Type::Composite(CompositeType::StructuredArray(
            ScalarType::F32,
        )))?
        .build();

    saxpy = saxpy
        .param(3)
        .set_extern_pointer(true)
        .set_kind(ParameterKind::DescriptorSet {
            storage_class: StorageClass::StorageBuffer,
            set: 0,
            binding: 3,
        })?
        .set_type(Type::Composite(CompositeType::StructuredArray(
            ScalarType::F32,
        )))?
        .build();

    saxpy.build();

    let config = config.build();

    let mut res = Cursor::new(Vec::<u8>::new());
    config.serialize_into(&mut res)?;
    println!("{res:?}");

    Ok(())
}
*/
