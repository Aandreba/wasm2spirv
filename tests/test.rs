use rspirv::{
    binary::{Assemble, Disassemble},
    spirv::{ExecutionModel, MemoryModel, StorageClass},
};
use std::mem::ManuallyDrop;
use wasm2spirv_lib::{
    ast::{
        function::{ExecutionMode, ParameterKind},
        module::ModuleBuilder,
    },
    config::{AddressingModel, CapabilityModel, Config, ExtensionModel, WasmFeatures},
    r#type::{CompositeType, ScalarType, Type},
    version::TargetPlatform,
};

#[test]
fn test() -> color_eyre::Result<()> {
    let _ = color_eyre::install();

    let mut config = Config::builder(
        TargetPlatform::VK_1_1,
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

    // let mut writer = File::create("saxpy_config.json")?;
    // serde_json::to_writer_pretty(&mut writer, &config)?;

    //let wat = include_str!("saxpy.wat");
    //let wasm = wat::parse_str(include_str!("../examples/saxpy.wat"))?;

    let wasm = wat::parse_bytes(include_bytes!("../examples/saxpy.wat"))?;
    let module = ModuleBuilder::new(config, &wasm)?;
    let spirv = module.translate().unwrap();

    let module = spirv.module();
    println!("{}", module.disassemble());

    let mut content = ManuallyDrop::new(module.assemble());
    let content = unsafe {
        Vec::from_raw_parts(
            content.as_mut_ptr().cast::<u8>(),
            4 * content.len(),
            4 * content.capacity(),
        )
    };

    std::fs::write("examples/out/test.spv", content)?;
    return Ok(());
}
