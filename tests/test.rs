use rspirv::{
    binary::Disassemble,
    spirv::{ExecutionModel, MemoryModel, StorageClass},
};
use wasm2spirv::{
    ast::{function::ParameterKind, module::ModuleBuilder},
    config::{AddressingModel, CapabilityModel, Config},
    r#type::{CompositeType, ScalarType},
};

#[test]
fn test() -> color_eyre::Result<()> {
    let _ = color_eyre::install();

    let mut config = Config::builder(
        CapabilityModel::default(),
        AddressingModel::Logical,
        MemoryModel::Simple,
    )?;

    // Add two
    config
        .function(0)
        .set_entry_point(ExecutionModel::GLCompute)?
        .set_param(1)
        .set_type(CompositeType::structured_array(ScalarType::I32))?
        .set_kind(ParameterKind::DescriptorSet {
            storage_class: StorageClass::StorageBuffer,
            set: 0,
            binding: 0,
        })?
        .build()
        .set_param(2)
        .set_type(CompositeType::structured_array(ScalarType::I32))?
        .set_kind(ParameterKind::DescriptorSet {
            storage_class: StorageClass::StorageBuffer,
            set: 0,
            binding: 1,
        })?
        .build()
        .build();

    let config = config.build();

    //let wat = include_str!("saxpy.wat");
    let wasm = wat::parse_str(include_str!("../simple.wat")).unwrap();
    let module = ModuleBuilder::new(config, &wasm).unwrap();
    let spirv = module.translate().unwrap();
    println!("{}", spirv.module().disassemble());
    return Ok(());
}
