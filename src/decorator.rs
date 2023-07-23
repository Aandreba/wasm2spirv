use rspirv::{
    dr::Operand,
    spirv::{BuiltIn, Decoration},
};

#[derive(Debug, Clone, PartialEq)]
pub enum VariableDecorator {
    BuiltIn(BuiltIn),
    DesctiptorSet(u32),
    Binding(u32),
    Location(u32),
    Flat,
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

impl VariableDecorator {
    pub fn translate(&self, target: rspirv::spirv::Word, builder: &mut rspirv::dr::Builder) {
        match self {
            VariableDecorator::BuiltIn(x) => {
                builder.decorate(target, Decoration::BuiltIn, [Operand::BuiltIn(*x)])
            }
            VariableDecorator::DesctiptorSet(x) => builder.decorate(
                target,
                Decoration::DescriptorSet,
                [Operand::LiteralInt32(*x)],
            ),
            VariableDecorator::Binding(x) => {
                builder.decorate(target, Decoration::Binding, [Operand::LiteralInt32(*x)])
            }
            VariableDecorator::Location(x) => {
                builder.decorate(target, Decoration::Location, [Operand::LiteralInt32(*x)])
            }
            VariableDecorator::Flat => builder.decorate(target, Decoration::Flat, None),
        }
    }
}
