use rspirv::{
    binary::Disassemble,
    spirv::{ExecutionModel, MemoryModel, StorageClass},
};
use wasm2spirv::{
    ast::{
        function::{ExecutionMode, ParameterKind},
        module::ModuleBuilder,
    },
    config::{AddressingModel, CapabilityModel, Config},
    r#type::{CompositeType, ScalarType, Type},
};
use wasmparser::WasmFeatures;

#[test]
fn test() -> color_eyre::Result<()> {
    let _ = color_eyre::install();

    let mut config = Config::builder(
        CapabilityModel::default(),
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

    saxpy = saxpy.param(0).set_kind(ParameterKind::Input)?.build();
    saxpy = saxpy.param(1).set_kind(ParameterKind::Input)?.build();

    saxpy = saxpy
        .param(2)
        .set_extern_pointer(true)
        .set_kind(ParameterKind::DescriptorSet {
            storage_class: StorageClass::StorageBuffer,
            set: 0,
            binding: 0,
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
            binding: 1,
        })?
        .set_type(Type::Composite(CompositeType::StructuredArray(
            ScalarType::F32,
        )))?
        .build();

    saxpy.build();
    let config = config.build();

    //let wat = include_str!("saxpy.wat");
    let wasm = wat::parse_str(include_str!("../saxpy.wat"))?;
    let module = ModuleBuilder::new(config, &wasm)?;
    let spirv = module.translate()?;
    println!("{}", spirv.module().disassemble());
    return Ok(());
}
