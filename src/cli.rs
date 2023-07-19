use clap::{Parser, ValueEnum};
use color_eyre::Report;
use spirv::{ExecutionModel, MemoryModel, StorageClass};
use std::{fs::File, io::Cursor, path::PathBuf};
use wasm2spirv_core::{
    ast::{
        function::{ExecutionMode, ParameterKind},
        *,
    },
    binary::BinarySerialize,
    config::{AddressingModel, CapabilityModel, Config, ExtensionModel, WasmFeatures},
    r#type::{CompositeType, ScalarType, Type},
    version::TargetPlatform,
};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// File to be converted. Has to be a WebAssembly text or binary file
    target: PathBuf,

    /// Import compilation fonciguration from a custom section on the WebAssemly program itself
    #[arg(long, default_value_t = false)]
    from_file: bool,

    /// Import compilation configuration from JSON file
    #[arg(long)]
    from_json: Option<PathBuf>,
}

pub fn main() -> color_eyre::Result<()> {
    let _ = color_eyre::install();
    tracing_subscriber::fmt::try_init().map_err(Report::msg)?;

    let cli: Cli = Cli::parse();
    let config: Config;

    if let Some(json) = cli.from_json {
        let mut file = File::open(json)?;
        config = serde_json::from_reader(&mut file)?;
    } else if cli.from_file {
        todo!()
    } else {
        todo!()
    }

    return Ok(());
}

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
