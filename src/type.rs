use crate::{
    ast::{
        module::ModuleBuilder,
        values::{float::FloatKind, integer::IntegerKind},
    },
    config::storage_class_capability,
};
use rspirv::spirv::{Capability, StorageClass};
use wasmparser::ValType;

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Type {
    Pointer(StorageClass, Box<Type>),
    Scalar(ScalarType),
    Composite(CompositeType),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScalarType {
    I32,
    I64,
    F32,
    F64,
    Bool,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum CompositeType {
    StructuredArray(ScalarType),
    Vector(ScalarType, u32),
}

impl Type {
    pub fn pointer(storage_class: StorageClass, ty: impl Into<Type>) -> Type {
        Self::Pointer(storage_class, Box::new(ty.into()))
    }

    pub fn required_capabilities(&self) -> Vec<Capability> {
        match self {
            Type::Pointer(storage_class, pointee) => {
                let mut res = pointee.required_capabilities();
                res.extend(storage_class_capability(*storage_class));
                res
            }
            Type::Scalar(x) => x.required_capabilities(),
            Type::Composite(x) => x.required_capabilities(),
        }
    }

    pub fn comptime_byte_size(&self, module: &ModuleBuilder) -> Option<u32> {
        match self {
            Type::Pointer(storage_class, _) => module.spirv_address_bytes(*storage_class),
            Type::Scalar(x) => x.byte_size(),
            Type::Composite(CompositeType::StructuredArray(_)) => None,
            Type::Composite(CompositeType::Vector(elem, count)) => Some(elem.byte_size()? * count),
        }
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
    pub fn structured_array(elem: impl Into<ScalarType>) -> CompositeType {
        return CompositeType::StructuredArray(elem.into());
    }

    pub fn vector(elem: impl Into<ScalarType>, count: u32) -> CompositeType {
        return CompositeType::Vector(elem.into(), count);
    }

    pub fn required_capabilities(&self) -> Vec<Capability> {
        match self {
            CompositeType::StructuredArray(elem) => {
                let mut res = vec![Capability::Shader];
                res.extend(elem.required_capabilities());
                res
            }
            CompositeType::Vector(elem, _) => elem.required_capabilities(),
        }
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
