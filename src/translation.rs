use crate::{
    ast::{
        function::{Schrodinger, SchrodingerKind, Storeable},
        module::ModuleBuilder,
        values::{
            float::{
                BinarySource as FloatBinarySource, ConstantSource as FloatConstantSource,
                ConversionSource as FloatConversionSource, Float, FloatKind, FloatSource,
                UnarySource as FloatUnarySource,
            },
            integer::{
                BinarySource as IntBinarySource, ConstantSource as IntConstantSource,
                ConversionSource as IntConversionSource, Integer, IntegerKind, IntegerSource,
                UnarySource as IntUnarySource,
            },
            pointer::{Pointer, PointerSource},
            Value,
        },
        Operation,
    },
    error::{Error, Result},
    r#type::{CompositeType, ScalarType, Type},
};
use rspirv::{
    dr::{Module, Operand},
    spirv::{Decoration, FunctionControl, MemoryAccess, StorageClass},
};
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    rc::Rc,
};
use tracing::warn;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Constant {
    U32(u32),
    U64(u64),
    F32(u32),
    F64(u64),
}

pub struct Builder {
    inner: rspirv::dr::Builder,
    constants: HashMap<(rspirv::spirv::Word, Constant), rspirv::spirv::Word>,
}

impl Builder {
    pub fn new() -> Self {
        return Self {
            inner: rspirv::dr::Builder::new(),
            constants: HashMap::new(),
        };
    }

    pub fn module(self) -> Module {
        self.inner.module()
    }

    pub fn constant_u32(
        &mut self,
        result_type: rspirv::spirv::Word,
        value: u32,
    ) -> rspirv::spirv::Word {
        *self
            .constants
            .entry((result_type, Constant::U32(value)))
            .or_insert_with(|| self.inner.constant_u32(result_type, value))
    }

    pub fn constant_u64(
        &mut self,
        result_type: rspirv::spirv::Word,
        value: u64,
    ) -> rspirv::spirv::Word {
        *self
            .constants
            .entry((result_type, Constant::U64(value)))
            .or_insert_with(|| self.inner.constant_u64(result_type, value))
    }

    pub fn constant_f32(
        &mut self,
        result_type: rspirv::spirv::Word,
        value: f32,
    ) -> rspirv::spirv::Word {
        *self
            .constants
            .entry((result_type, Constant::F32(f32::to_bits(value))))
            .or_insert_with(|| self.inner.constant_f32(result_type, value))
    }

    pub fn constant_f64(
        &mut self,
        result_type: rspirv::spirv::Word,
        value: f64,
    ) -> rspirv::spirv::Word {
        *self
            .constants
            .entry((result_type, Constant::F64(f64::to_bits(value))))
            .or_insert_with(|| self.inner.constant_f64(result_type, value))
    }
}

impl ModuleBuilder {
    pub fn translate(self) -> Result<Builder> {
        let mut builder = Builder::new();

        // Capabilities
        for capability in &self.capabilities {
            builder.capability(*capability)
        }

        // TODO extensions

        // TODO extended instruction sets

        // Memory model
        builder.memory_model(self.addressing_model, self.memory_model);

        // TODO entry points

        // TODO debug info

        // TODO anotations

        // Function declarations
        for function in self.built_functions.iter() {
            function.function_id.set(Some(builder.id()));
        }

        for function in self.built_functions.iter() {
            let return_type = match &function.return_type {
                Some(ty) => ty.clone().translate(&self, &mut builder)?,
                None => builder.type_void(),
            };
            let parameters = function
                .parameters
                .iter()
                .cloned()
                .map(|x| x.translate(&self, &mut builder))
                .collect::<Result<Vec<_>, _>>()?;
            let function_type = builder.type_function(return_type, parameters);

            builder.begin_function(
                return_type,
                function.function_id.get(),
                FunctionControl::NONE,
                function_type,
            )?;

            // TODO Initialize function parameters

            builder.begin_block(None)?;

            // Initialize local variables
            for var in function.local_variables.iter() {
                let _ = var.storeable.translate(&self, &mut builder)?;
            }

            // Translate anchors
            for anchor in function.anchors.iter() {
                let _ = anchor.translate(&self, &mut builder)?;
            }

            builder.end_function()?;
        }

        return Ok(builder);
    }
}

pub trait Translation {
    fn translate(
        self,
        module: &ModuleBuilder,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word>;
}

/* TYPES */
impl Translation for ScalarType {
    fn translate(self, _: &ModuleBuilder, builder: &mut Builder) -> Result<rspirv::spirv::Word> {
        return Ok(match self {
            ScalarType::I32 => builder.type_int(32, 0),
            ScalarType::I64 => builder.type_int(64, 0),
            ScalarType::F32 => builder.type_float(32),
            ScalarType::F64 => builder.type_float(64),
        });
    }
}

impl Translation for CompositeType {
    fn translate(
        self,
        module: &ModuleBuilder,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        match self {
            CompositeType::StructuredArray(elem) => {
                let element = elem.translate(module, builder)?;

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
    fn translate(
        self,
        module: &ModuleBuilder,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        match self {
            Type::Pointer(storage_class, pointee) => {
                let pointee_type = pointee.translate(module, builder)?;
                Ok(builder.type_pointer(None, storage_class, pointee_type))
            }
            Type::Scalar(x) => x.translate(module, builder),
            Type::Composite(x) => x.translate(module, builder),
        }
    }
}

/* OPERATIONS */
impl Translation for &Schrodinger {
    fn translate(
        self,
        module: &ModuleBuilder,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        if let Some(res) = self.translation.get() {
            return Ok(res);
        }

        let ty = match self.kind.get() {
            Some(SchrodingerKind::Integer) => module.isize_type().into(),
            Some(SchrodingerKind::Pointer(storage_class, pointee)) => {
                Type::pointer(*storage_class, pointee.clone())
            }
            None => {
                warn!("Underlying type for schrodinger variable is still unknown. Defaulting to integer.");
                module.isize_type().into()
            }
        };

        let pointee_type = ty.translate(module, builder)?;
        let result_type = builder.type_pointer(None, StorageClass::Function, pointee_type);
        let res = builder.variable(result_type, None, StorageClass::Function, None);

        self.translation.set(Some(res));
        return Ok(res);
    }
}

impl Translation for &Integer {
    fn translate(
        self,
        module: &ModuleBuilder,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        if let Some(res) = self.translation.get() {
            return Ok(res);
        }

        let result_type = builder.type_int(
            match self.kind(module)? {
                IntegerKind::Short => 32,
                IntegerKind::Long => 64,
            },
            0,
        );

        let res = match &self.source {
            IntegerSource::FunctionParam(_) => builder.function_parameter(result_type),

            IntegerSource::ArrayLength { structured_array } => {
                let structure = structured_array.translate(module, builder)?;
                builder.array_length(result_type, None, structure, 0)
            }

            IntegerSource::Constant(IntConstantSource::Short(x)) => {
                Ok(builder.constant_u32(result_type, *x))
            }

            IntegerSource::Constant(IntConstantSource::Long(x)) => {
                Ok(builder.constant_u64(result_type, *x))
            }

            IntegerSource::Conversion(
                IntConversionSource::FromLong(value)
                | IntConversionSource::FromShort {
                    signed: false,
                    value,
                },
            ) => {
                let unsigned_value = value.translate(module, builder)?;
                builder.u_convert(result_type, None, unsigned_value)
            }

            IntegerSource::Conversion(IntConversionSource::FromShort {
                signed: true,
                value,
            }) => {
                let unsigned_value = value.translate(module, builder)?;
                builder.s_convert(result_type, None, unsigned_value)
            }

            IntegerSource::Conversion(IntConversionSource::FromPointer(pointer)) => {
                let pointer = pointer.translate(module, builder)?;
                builder.convert_ptr_to_u(result_type, None, pointer)
            }

            IntegerSource::Loaded {
                pointer,
                log2_alignment,
            } => {
                let pointer = pointer.translate(module, builder)?;
                let (memory_access, additional_params) = additional_access_info(*log2_alignment);
                builder.load(result_type, None, pointer, memory_access, additional_params)
            }

            IntegerSource::FunctionCall { args, kind } => todo!(),

            IntegerSource::Unary { source, op1 } => {
                let operand = op1.translate(module, builder)?;
                match source {
                    IntUnarySource::Not => builder.not(result_type, None, operand),
                    IntUnarySource::Negate => builder.s_negate(result_type, None, operand),
                }
            }

            IntegerSource::Binary { source, op1, op2 } => {
                let operand_1 = op1.translate(module, builder)?;
                let operand_2 = op2.translate(module, builder)?;
                match source {
                    IntBinarySource::Add => builder.i_add(result_type, None, operand_1, operand_2),
                    IntBinarySource::Sub => builder.i_sub(result_type, None, operand_1, operand_2),
                    IntBinarySource::Mul => builder.i_mul(result_type, None, operand_1, operand_2),
                    IntBinarySource::SDiv => builder.s_div(result_type, None, operand_1, operand_2),
                    IntBinarySource::UDiv => builder.u_div(result_type, None, operand_1, operand_2),
                    IntBinarySource::SRem => builder.s_rem(result_type, None, operand_1, operand_2),
                    IntBinarySource::URem => builder.u_mod(result_type, None, operand_1, operand_2),
                    IntBinarySource::And => {
                        builder.bitwise_and(result_type, None, operand_1, operand_2)
                    }
                    IntBinarySource::Or => {
                        builder.bitwise_or(result_type, None, operand_1, operand_2)
                    }
                    IntBinarySource::Xor => {
                        builder.bitwise_xor(result_type, None, operand_1, operand_2)
                    }
                    IntBinarySource::Shl => {
                        builder.shift_left_logical(result_type, None, operand_1, operand_2)
                    }
                    IntBinarySource::SShr => {
                        builder.shift_right_arithmetic(result_type, None, operand_1, operand_2)
                    }
                    IntBinarySource::UShr => {
                        builder.shift_right_logical(result_type, None, operand_1, operand_2)
                    }
                }
            }
        }?;

        self.translation.set(Some(res));
        return Ok(res);
    }
}

impl Translation for &Float {
    fn translate(
        self,
        module: &ModuleBuilder,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        if let Some(res) = self.translation.get() {
            return Ok(res);
        }

        let result_type = builder.type_float(match self.kind()? {
            FloatKind::Single => 32,
            FloatKind::Double => 64,
        });

        let res = match &self.source {
            FloatSource::FunctionParam(_) => builder.function_parameter(result_type),
            FloatSource::Constant(FloatConstantSource::Single(x)) => {
                Ok(builder.constant_f32(result_type, *x))
            }
            FloatSource::Constant(FloatConstantSource::Double(x)) => {
                Ok(builder.constant_f64(result_type, *x))
            }
            FloatSource::Conversion(
                FloatConversionSource::FromDouble(value) | FloatConversionSource::FromSingle(value),
            ) => {
                let float_value = value.translate(module, builder)?;
                builder.f_convert(result_type, None, float_value)
            }
            FloatSource::Loaded {
                pointer,
                log2_alignment,
            } => {
                let pointer = pointer.translate(module, builder)?;
                let (memory_access, additional_params) = additional_access_info(*log2_alignment);
                builder.load(result_type, None, pointer, memory_access, additional_params)
            }
            FloatSource::FunctionCall { args, kind } => todo!(),
            FloatSource::Unary { source, op1 } => {
                let operand = op1.translate(module, builder)?;
                match source {
                    FloatUnarySource::Negate => builder.f_negate(result_type, None, operand),
                }
            }
            FloatSource::Binary { source, op1, op2 } => {
                let operand_1 = op1.translate(module, builder)?;
                let operand_2 = op2.translate(module, builder)?;
                match source {
                    FloatBinarySource::Add => {
                        builder.f_add(result_type, None, operand_1, operand_2)
                    }
                    FloatBinarySource::Sub => {
                        builder.f_sub(result_type, None, operand_1, operand_2)
                    }
                    FloatBinarySource::Mul => {
                        builder.f_mul(result_type, None, operand_1, operand_2)
                    }
                    FloatBinarySource::Div => {
                        builder.f_div(result_type, None, operand_1, operand_2)
                    }
                    FloatBinarySource::Sqrt => todo!(),
                }
            }
        }?;

        self.translation.set(Some(res));
        return Ok(res);
    }
}

impl Translation for &Rc<Pointer> {
    fn translate(
        self,
        module: &ModuleBuilder,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        if let Some(res) = self.translation.get() {
            return Ok(res);
        }

        let pointer_type = self.pointer_type().translate(module, builder)?;
        let res = match &self.source {
            PointerSource::FunctionParam => builder.function_parameter(pointer_type),

            PointerSource::Casted { prev } => {
                let prev = prev.translate(module, builder)?;
                builder.bitcast(pointer_type, None, prev)
            }

            PointerSource::FromInteger(value) => {
                let integer_value = value.translate(module, builder)?;
                builder.convert_u_to_ptr(pointer_type, None, integer_value)
            }

            PointerSource::Loaded {
                pointer,
                log2_alignment,
            } => {
                let pointer = pointer.translate(module, builder)?;
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

            PointerSource::Access { base, byte_element } => {
                let base_pointee = &base.pointee;
                let base = base.translate(module, builder)?;

                match base_pointee {
                    Type::Composite(CompositeType::StructuredArray(elem)) => {
                        // Downscale element from byte-sized to element-sized
                        let element_size =
                            Rc::new(Integer::new_constant_usize(elem.byte_size(), module));
                        let element = byte_element
                            .clone()
                            .u_div(element_size, module)?
                            .translate(module, builder)?;

                        let result_type =
                            Type::pointer(self.storage_class, **elem).translate(module, builder)?;
                        let zero =
                            Integer::new_constant_usize(0, module).translate(module, builder)?;

                        builder.access_chain(result_type, None, base, [zero, element])
                    }
                    _ => {
                        let element_size = self
                            .clone()
                            .pointee_byte_size(module)
                            .map(Rc::new)
                            .ok_or_else(|| Error::msg("Pointed element has no size"))?;

                        let element = byte_element
                            .clone()
                            .u_div(element_size, module)?
                            .translate(module, builder)?;

                        builder.ptr_access_chain(pointer_type, None, base, element, None)
                    }
                }
            }

            PointerSource::Variable { init, decorators } => {
                let initializer = init
                    .as_ref()
                    .map(|x| x.translate(module, builder))
                    .transpose()?;
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

impl Translation for &Storeable {
    fn translate(
        self,
        module: &ModuleBuilder,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        return match self {
            Storeable::Pointer(x) => x.translate(module, builder),
            Storeable::Schrodinger(x) => x.translate(module, builder),
        };
    }
}

impl Translation for &Value {
    fn translate(
        self,
        module: &ModuleBuilder,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        match self {
            Value::Integer(x) => x.translate(module, builder),
            Value::Float(x) => x.translate(module, builder),
            Value::Pointer(x) => x.translate(module, builder),
        }
    }
}

impl Translation for &Operation {
    fn translate(
        self,
        module: &ModuleBuilder,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        match self {
            Operation::Value(x) => return x.translate(module, builder),
            Operation::Store {
                target: pointer,
                value,
                log2_alignment,
            } => {
                let pointer = pointer.translate(module, builder)?;
                let object = value.translate(module, builder)?;
                let (memory_access, additional_params) = log2_alignment
                    .map(|align| (MemoryAccess::ALIGNED, Operand::LiteralInt32(1 << align)))
                    .unzip();

                builder.store(pointer, object, memory_access, additional_params)
            }
            Operation::FunctionCall { args } => {
                let args = args
                    .iter()
                    .map(|x| x.translate(module, builder))
                    .collect::<Result<Vec<_>, _>>()?;

                let void = builder.type_void();
                builder.function_call(void, None, todo!(), args)?;
            }
            Operation::Nop => builder.nop(),
            Operation::Unreachable => builder.unreachable(),
            Operation::End {
                return_value: Some(value),
            } => {
                let value = value.translate(module, builder)?;
                builder.ret_value(value)
            }
            Operation::End { return_value: None } => todo!(),
        }?;

        return Ok(0);
    }
}

impl Deref for Builder {
    type Target = rspirv::dr::Builder;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Builder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

fn additional_access_info(log2_alignment: Option<u32>) -> (Option<MemoryAccess>, Option<Operand>) {
    log2_alignment
        .map(|align| (MemoryAccess::ALIGNED, Operand::LiteralInt32(1 << align)))
        .unzip()
}
