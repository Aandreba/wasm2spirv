#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Type {
    Scalar(ScalarType),
    Composite(CompositeType),
    Schrodinger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScalarType {
    I32,
    I64,
    F32,
    F64,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum CompositeType {
    StructuredArray(Box<ScalarType>),
}

impl Type {
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
    pub fn byte_size(self) -> u32 {
        match self {
            ScalarType::I32 | ScalarType::F32 => 4,
            ScalarType::I64 | ScalarType::F64 => 8,
        }
    }
}
