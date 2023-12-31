use crate::fg::{
    module::ModuleBuilder,
    values::{float::FloatKind, integer::IntegerKind, pointer::PointerKind},
};
use num_enum::TryFromPrimitive;
use rspirv::spirv::{Capability, StorageClass};
use serde::{Deserialize, Serialize};
use wasmparser::ValType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PointerSize {
    #[default]
    Skinny,
    Fat,
}

impl PointerSize {
    pub fn to_pointer_kind(self) -> PointerKind {
        match self {
            PointerSize::Skinny => PointerKind::skinny(),
            PointerSize::Fat => PointerKind::fat(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Type {
    Pointer {
        size: PointerSize,
        storage_class: StorageClass,
        pointee: Box<Type>,
    },
    Scalar(ScalarType),
    Composite(CompositeType),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[repr(u16)]
pub enum ScalarType {
    I32,
    I64,
    F32,
    F64,
    Bool,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
pub enum CompositeType {
    Vector(ScalarType, u32),
}

impl Type {
    pub fn pointer(size: PointerSize, storage_class: StorageClass, ty: impl Into<Type>) -> Type {
        Self::Pointer {
            size,
            storage_class,
            pointee: Box::new(ty.into()),
        }
    }

    pub fn comptime_byte_size(&self, module: &ModuleBuilder) -> Option<u32> {
        match self {
            Type::Pointer { storage_class, .. } => module.spirv_address_bytes(*storage_class),
            Type::Scalar(x) => x.byte_size(),
            Type::Composite(CompositeType::Vector(elem, count)) => Some(elem.byte_size()? * count),
        }
    }

    pub fn is_pointer(&self) -> bool {
        return matches!(self, Self::Pointer { .. });
    }

    pub fn is_scalar(&self) -> bool {
        return self.get_scalar().is_some();
    }

    pub fn is_composite(&self) -> bool {
        return self.get_composite().is_some();
    }

    pub fn get_scalar(&self) -> Option<&ScalarType> {
        match self {
            Type::Scalar(scalar) => Some(scalar),
            _ => None,
        }
    }

    pub fn get_composite(&self) -> Option<&CompositeType> {
        match self {
            Type::Composite(composite) => Some(composite),
            _ => None,
        }
    }
}

impl ScalarType {
    #[allow(non_snake_case)]
    pub fn Isize(module: &ModuleBuilder) -> Self {
        match module.wasm_memory64 {
            true => ScalarType::I64,
            false => ScalarType::I32,
        }
    }

    pub fn required_capabilities(&self) -> Vec<Capability> {
        match self {
            ScalarType::Bool | ScalarType::I32 | ScalarType::F32 => Vec::new(),
            ScalarType::I64 => vec![Capability::Int64],
            ScalarType::F64 => vec![Capability::Float64],
        }
    }

    pub fn byte_size(self) -> Option<u32> {
        match self {
            ScalarType::Bool => None,
            ScalarType::I32 | ScalarType::F32 => Some(4),
            ScalarType::I64 | ScalarType::F64 => Some(8),
        }
    }
}

impl CompositeType {
    pub fn vector(elem: impl Into<ScalarType>, count: u32) -> CompositeType {
        return CompositeType::Vector(elem.into(), count);
    }
}

/* CONVERSIONS */
impl From<IntegerKind> for ScalarType {
    fn from(value: IntegerKind) -> Self {
        match value {
            IntegerKind::Short => ScalarType::I32,
            IntegerKind::Long => ScalarType::I64,
        }
    }
}

impl From<FloatKind> for ScalarType {
    fn from(value: FloatKind) -> Self {
        match value {
            FloatKind::Single => ScalarType::F32,
            FloatKind::Double => ScalarType::F64,
        }
    }
}

impl From<IntegerKind> for Type {
    fn from(value: IntegerKind) -> Self {
        Type::Scalar(value.into())
    }
}

impl From<FloatKind> for Type {
    fn from(value: FloatKind) -> Self {
        Type::Scalar(value.into())
    }
}

impl From<ScalarType> for Type {
    fn from(value: ScalarType) -> Self {
        Self::Scalar(value)
    }
}

impl From<CompositeType> for Type {
    fn from(value: CompositeType) -> Self {
        Self::Composite(value)
    }
}

impl From<ValType> for Type {
    fn from(value: ValType) -> Self {
        match value {
            ValType::I32 => Type::Scalar(ScalarType::I32),
            ValType::I64 => Type::Scalar(ScalarType::I64),
            ValType::F32 => Type::Scalar(ScalarType::F32),
            ValType::F64 => Type::Scalar(ScalarType::F64),
            ValType::V128 => todo!(),
            ValType::Ref(_) => todo!(),
        }
    }
}
