use rspirv::dr::{Instruction, Operand};
use spirv::{
    AddressingModel, BuiltIn, Capability, ExecutionMode, ExecutionModel, MemoryModel, StorageClass,
};
use tracing::warn;

pub fn instruction_capabilities<'a>(
    instr: &'a Instruction,
) -> impl 'a + Iterator<Item = Capability> {
    let opcode = instr.class.capabilities.iter().copied();
    let operands = instr.operands.iter().flat_map(operand_capabilities);
    opcode.chain(operands)
}

fn operand_capabilities(op: &Operand) -> Vec<Capability> {
    match op {
        Operand::AddressingModel(addressing_model) => {
            addressing_model_capabilities(*addressing_model)
        }
        Operand::BuiltIn(builtin) => builtin_capabilities(*builtin),
        Operand::StorageClass(storage_class) => storage_class_capabilities(*storage_class),
        Operand::MemoryModel(memory_model) => memory_model_capabilities(*memory_model),
        Operand::ExecutionModel(execution_model) => execution_model_capabilities(*execution_model),
        Operand::ExecutionMode(execution_mode) => execution_mode_capabilities(*execution_mode),
        Operand::Decoration(_)
        | Operand::IdRef(_)
        | Operand::LiteralInt32(_)
        | Operand::LiteralInt64(_)
        | Operand::LiteralFloat32(_)
        | Operand::LiteralFloat64(_)
        | Operand::LiteralExtInstInteger(_)
        | Operand::LiteralSpecConstantOpInteger(_)
        | Operand::LiteralString(_) => Vec::new(),
        other => {
            warn!("Not yet implemented operand: {other:?}");
            Vec::new()
        }
    }
}

fn storage_class_capabilities(storage_class: StorageClass) -> Vec<Capability> {
    return match storage_class {
        StorageClass::Uniform
        | StorageClass::Output
        | StorageClass::Private
        | StorageClass::PushConstant
        | StorageClass::StorageBuffer => vec![Capability::Shader],
        StorageClass::PhysicalStorageBuffer => vec![Capability::PhysicalStorageBufferAddresses],
        StorageClass::AtomicCounter => vec![Capability::AtomicStorage],
        StorageClass::Generic => vec![Capability::GenericPointer],
        other => {
            warn!("Not yet implemented storage class: {other:?}");
            return Vec::new();
        }
    };
}

fn addressing_model_capabilities(addressing_model: AddressingModel) -> Vec<Capability> {
    match addressing_model {
        AddressingModel::Logical => Vec::new(),
        AddressingModel::Physical32 | AddressingModel::Physical64 => vec![Capability::Addresses],
        AddressingModel::PhysicalStorageBuffer64 => {
            vec![Capability::PhysicalStorageBufferAddresses]
        }
    }
}

fn memory_model_capabilities(memory_model: MemoryModel) -> Vec<Capability> {
    match memory_model {
        MemoryModel::Simple | MemoryModel::GLSL450 => vec![Capability::Shader],
        MemoryModel::OpenCL => vec![Capability::Kernel],
        MemoryModel::Vulkan => vec![Capability::VulkanMemoryModel],
    }
}

fn execution_mode_capabilities(execution_mode: ExecutionMode) -> Vec<Capability> {
    match execution_mode {
        ExecutionMode::Invocations => vec![Capability::Geometry],
        ExecutionMode::PixelCenterInteger
        | ExecutionMode::DepthReplacing
        | ExecutionMode::OriginUpperLeft
        | ExecutionMode::OriginLowerLeft => vec![Capability::Shader],
        ExecutionMode::LocalSizeHint => vec![Capability::Kernel],
        ExecutionMode::LocalSize => Vec::new(),
        other => {
            warn!("Not yet implemented execution mode: {other:?}");
            return Vec::new();
        }
    }
}

fn execution_model_capabilities(execution_model: ExecutionModel) -> Vec<Capability> {
    match execution_model {
        ExecutionModel::Fragment | ExecutionModel::GLCompute | ExecutionModel::Vertex => {
            vec![Capability::Shader]
        }
        ExecutionModel::TessellationEvaluation | ExecutionModel::TessellationControl => {
            vec![Capability::Tessellation]
        }
        ExecutionModel::Geometry => vec![Capability::Geometry],
        ExecutionModel::Kernel => vec![Capability::Kernel],
        _ => Vec::new(),
    }
}

fn builtin_capabilities(builtin: BuiltIn) -> Vec<Capability> {
    match builtin {
        BuiltIn::Position
        | BuiltIn::PointSize
        | BuiltIn::VertexId
        | BuiltIn::InstanceId
        | BuiltIn::FragCoord
        | BuiltIn::PointCoord
        | BuiltIn::SampleMask
        | BuiltIn::FragDepth
        | BuiltIn::HelperInvocation => vec![Capability::Shader],
        other => {
            warn!("Not yet implemented built-in: {other:?}");
            Vec::new()
        }
    }
}
