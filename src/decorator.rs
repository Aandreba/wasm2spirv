use rspirv::spirv::BuiltIn;

#[derive(Debug, Clone, PartialEq)]
pub enum VariableDecorator {
    BuiltIn(BuiltIn),
    DesctiptorSet(u32),
    Binding(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeDecorator {
    Block,
    BufferBlock,
    ArrayStride(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeMemberDecorator {
    Offset(u32),
    NonWriteable,
}
