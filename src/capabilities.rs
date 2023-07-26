use rspirv::dr::{Instruction, Operand};
use spirv::{
    AddressingModel, BuiltIn, Capability, ExecutionMode, ExecutionModel, FunctionControl,
    MemoryAccess, MemoryModel, StorageClass,
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
    use Operand::*;

    match op {
        AddressingModel(addressing_model) => addressing_model_capabilities(*addressing_model),
        BuiltIn(builtin) => builtin_capabilities(*builtin),
        StorageClass(storage_class) => storage_class_capabilities(*storage_class),
        MemoryModel(memory_model) => memory_model_capabilities(*memory_model),
        ExecutionModel(execution_model) => execution_model_capabilities(*execution_model),
        ExecutionMode(execution_mode) => execution_mode_capabilities(*execution_mode),
        MemoryAccess(memory_access) => memory_access_capabilities(*memory_access),
        FunctionControl(control) => function_control_capabilities(*control),
        Decoration(_)
        | IdRef(_)
        | LiteralInt32(_)
        | LiteralInt64(_)
        | LiteralFloat32(_)
        | LiteralFloat64(_)
        | LiteralExtInstInteger(_)
        | LiteralSpecConstantOpInteger(_)
        | LiteralString(_) => Vec::new(),
        other => {
            warn!("Not yet implemented operand: {other:?}");
            Vec::new()
        }
    }
}

fn storage_class_capabilities(storage_class: StorageClass) -> Vec<Capability> {
    use StorageClass::*;

    return match storage_class {
        Uniform | Output | Private | PushConstant | StorageBuffer => vec![Capability::Shader],
        PhysicalStorageBuffer => vec![Capability::PhysicalStorageBufferAddresses],
        AtomicCounter => vec![Capability::AtomicStorage],
        Generic => vec![Capability::GenericPointer],
        Input | Function => Vec::new(),
        other => {
            warn!("Not yet implemented storage class: {other:?}");
            return Vec::new();
        }
    };
}

fn addressing_model_capabilities(addressing_model: AddressingModel) -> Vec<Capability> {
    use AddressingModel::*;

    match addressing_model {
        Logical => Vec::new(),
        Physical32 | Physical64 => vec![Capability::Addresses],
        PhysicalStorageBuffer64 => {
            vec![Capability::PhysicalStorageBufferAddresses]
        }
    }
}

fn memory_model_capabilities(memory_model: MemoryModel) -> Vec<Capability> {
    use MemoryModel::*;

    match memory_model {
        Simple | GLSL450 => vec![Capability::Shader],
        OpenCL => vec![Capability::Kernel],
        Vulkan => vec![Capability::VulkanMemoryModel],
    }
}

fn execution_mode_capabilities(execution_mode: ExecutionMode) -> Vec<Capability> {
    use ExecutionMode::*;

    match execution_mode {
        Invocations => vec![Capability::Geometry],
        PixelCenterInteger | DepthReplacing | OriginUpperLeft | OriginLowerLeft => {
            vec![Capability::Shader]
        }
        LocalSizeHint => vec![Capability::Kernel],
        LocalSize => Vec::new(),
        other => {
            warn!("Not yet implemented execution mode: {other:?}");
            return Vec::new();
        }
    }
}

fn execution_model_capabilities(execution_model: ExecutionModel) -> Vec<Capability> {
    use ExecutionModel::*;

    match execution_model {
        Fragment | GLCompute | Vertex => {
            vec![Capability::Shader]
        }
        TessellationEvaluation | TessellationControl => {
            vec![Capability::Tessellation]
        }
        Geometry => vec![Capability::Geometry],
        Kernel => vec![Capability::Kernel],
        _ => Vec::new(),
    }
}

fn builtin_capabilities(builtin: BuiltIn) -> Vec<Capability> {
    use BuiltIn::*;

    match builtin {
        Position | PointSize | VertexId | InstanceId | FragCoord | PointCoord | SampleMask
        | FragDepth | HelperInvocation => vec![Capability::Shader],
        NumWorkgroups | GlobalInvocationId => Vec::new(),
        other => {
            warn!("Not yet implemented built-in: {other:?}");
            Vec::new()
        }
    }
}

fn memory_access_capabilities(memory_access: MemoryAccess) -> Vec<Capability> {
    const VULKAN_MEMORY_MODEL: MemoryAccess = MemoryAccess::MAKE_POINTER_AVAILABLE
        .union(MemoryAccess::MAKE_POINTER_VISIBLE)
        .union(MemoryAccess::NON_PRIVATE_POINTER);

    let mut result = Vec::new();
    if memory_access.intersects(VULKAN_MEMORY_MODEL) {
        result.push(Capability::VulkanMemoryModel)
    }
    result
}

fn function_control_capabilities(_: FunctionControl) -> Vec<Capability> {
    Vec::new()
}
