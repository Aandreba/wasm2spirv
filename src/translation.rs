use crate::{
    ast::{
        block::BlockBuilder,
        values::{
            integer::{Integer, IntegerSource},
            pointer::{Pointer, PointerSource},
            Value,
        },
        Operation,
    },
    error::Result,
    r#type::{CompositeType, ScalarType, Type},
};
use rspirv::{
    dr::Operand,
    spirv::{Decoration, MemoryAccess},
};
use std::{ops::Deref, rc::Rc};

pub trait Translation {
    fn translate(self, builder: &mut rspirv::dr::Builder) -> Result<rspirv::spirv::Word>;
}

/* TYPES */
impl Translation for ScalarType {
    fn translate(self, builder: &mut rspirv::dr::Builder) -> Result<rspirv::spirv::Word> {
        return Ok(match self {
            ScalarType::I32 => builder.type_int(32, 0),
            ScalarType::I64 => builder.type_int(64, 0),
            ScalarType::F32 => builder.type_float(32),
            ScalarType::F64 => builder.type_float(64),
        });
    }
}

impl Translation for CompositeType {
    fn translate(self, builder: &mut rspirv::dr::Builder) -> Result<rspirv::spirv::Word> {
        match self {
            CompositeType::StructuredArray(elem) => {
                let element = elem.translate(builder)?;

                let types_global_values_len = builder.module_ref().types_global_values.len();
                let runtime_array = builder.type_runtime_array(element);
                if builder.module_ref().types_global_values.len() != types_global_values_len {
                    // Add decorators for runtime array
                    builder.decorate(
                        runtime_array,
                        Decoration::ArrayStride,
                        [Operand::LiteralInt32(elem.byte_size())],
                    );
                }

                let types_global_values_len = builder.module_ref().types_global_values.len();
                let structure = builder.type_struct(Some(runtime_array));
                if builder.module_ref().types_global_values.len() != types_global_values_len {
                    // Add decorators for struct
                    builder.member_decorate(
                        structure,
                        0,
                        Decoration::Offset,
                        [Operand::LiteralInt32(0)],
                    )
                }

                return Ok(structure);
            }
        }
    }
}

impl Translation for Type {
    fn translate(self, builder: &mut rspirv::dr::Builder) -> Result<rspirv::spirv::Word> {
        match self {
            Type::Pointer(storage_class, pointee) => {
                let pointee_type = pointee.translate(builder)?;
                Ok(builder.type_pointer(None, storage_class, pointee_type))
            }
            Type::Scalar(x) => x.translate(builder),
            Type::Composite(x) => x.translate(builder),
            Type::Schrodinger => todo!(),
        }
    }
}

/* OPERATIONS */
impl Translation for &Integer {
    fn translate(self, builder: &mut rspirv::dr::Builder) -> Result<rspirv::spirv::Word> {
        if let Some(res) = self.translation.get() {
            return Ok(res);
        }

        let res = match &self.source {
            IntegerSource::FunctionParam(_) => todo!(),
            IntegerSource::Constant(_) => todo!(),
            IntegerSource::Conversion(_) => todo!(),
            IntegerSource::Loaded { pointer } => todo!(),
            IntegerSource::FunctionCall { args, kind } => todo!(),
            IntegerSource::Unary { source, op1 } => todo!(),
            IntegerSource::Binary { source, op1, op2 } => todo!(),
        }?;

        self.translation.set(Some(res));
        return Ok(res);
    }
}

impl Translation for &Pointer {
    fn translate(self, builder: &mut rspirv::dr::Builder) -> Result<rspirv::spirv::Word> {
        if let Some(res) = self.translation.get() {
            return Ok(res);
        }

        let pointer_type = self.pointer_type().translate(builder)?;
        let res = match &self.source {
            PointerSource::FunctionParam => builder.function_parameter(pointer_type),
            PointerSource::Casted { prev } => {
                let prev = prev.translate(builder)?;
                builder.bitcast(pointer_type, None, prev)
            }
            PointerSource::FromInteger(_) => todo!(),
            PointerSource::Loaded {
                pointer,
                log2_alignment,
            } => {
                let pointer = pointer.translate(builder)?;
                let (memory_access, additional_params) = additional_access_info(*log2_alignment);
                builder.load(
                    pointer_type,
                    None,
                    pointer,
                    memory_access,
                    additional_params,
                )
            }
            PointerSource::FunctionCall { args } => todo!(),
            PointerSource::AccessChain { base, byte_indices } => {
                let element_size = self.element_bytes(module)?;
                let base = base.translate(builder)?;

                builder.access_chain(pointer_type, None, base, indexes)
            }
            PointerSource::PtrAccessChain {
                base,
                byte_element,
                byte_indices,
            } => todo!(),
            PointerSource::Variable { init, decorators } => {
                let initializer = init.map(|x| x.translate(builder)).transpose()?;
                let variable =
                    builder.variable(pointer_type, None, self.storage_class, initializer);

                decorators
                    .iter()
                    .for_each(|x| x.translate(variable, builder));
                Ok(variable)
            }
        }?;

        self.translation.set(Some(res));
        return Ok(res);
    }
}

impl Translation for Value {
    fn translate(self, builder: &mut rspirv::dr::Builder) -> Result<rspirv::spirv::Word> {
        match self {
            Value::Integer(_) => todo!(),
            Value::Float(_) => todo!(),
            Value::Pointer(x) => x.translate(builder),
            Value::Schrodinger(_) => todo!(),
        }
    }
}

impl Translation for Operation {
    fn translate(self, builder: &mut rspirv::dr::Builder) -> Result<rspirv::spirv::Word> {
        match self {
            Operation::Value(x) => return x.translate(builder),
            Operation::Store {
                pointer,
                value,
                log2_alignment,
            } => {
                let pointer = pointer.translate(builder)?;
                let (memory_access, additional_params) = log2_alignment
                    .map(|align| (MemoryAccess::ALIGNED, Operand::LiteralInt32(1 << align)))
                    .unzip();

                builder.store(pointer, object, memory_access, additional_params)
            }
            Operation::FunctionCall { args } => {
                let args = args
                    .into_vec()
                    .into_iter()
                    .map(|x| x.translate(builder))
                    .collect::<Result<Vec<_>, _>>()?;

                let void = builder.type_void();
                builder.function_call(void, None, todo!(), args)?;
            }
            Operation::Nop => builder.nop()?,
            Operation::Unreachable => builder.unreachable()?,
            Operation::End { return_value } => todo!(),
        };
        return Ok(0);
    }
}

/* BUILDERS */
impl<'a> Translation for BlockBuilder<'a> {
    fn translate(self, builder: &mut rspirv::dr::Builder) -> Result<rspirv::spirv::Word> {
        let prev_block = builder.selected_block();
        let id = builder.begin_block(None)?;

        for anchor in self.anchors {}

        builder.select_block(prev_block)?;
        return Ok(id);
    }
}

/* BLANKETS */
impl<T> Translation for Rc<T>
where
    for<'a> &'a T: Translation,
{
    #[inline]
    fn translate(self, builder: &mut rspirv::dr::Builder) -> Result<rspirv::spirv::Word> {
        self.deref().translate(builder)
    }
}

fn additional_access_info(log2_alignment: Option<u32>) -> (Option<MemoryAccess>, Option<Operand>) {
    log2_alignment
        .map(|align| (MemoryAccess::ALIGNED, Operand::LiteralInt32(1 << align)))
        .unzip()
}
