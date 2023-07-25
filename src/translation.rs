use crate::{
    error::{Error, Result},
    fg::{
        extended_is::{ExtendedSet, GLSLInstr, OpenCLInstr},
        function::{ExecutionMode, FunctionBuilder, Schrodinger, Storeable},
        module::{GlobalVariable, ModuleBuilder},
        values::{
            bool::{Bool, BoolSource, Comparison, Equality},
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
            pointer::{Pointer, PointerKind, PointerSource},
            vector::{Vector, VectorSource},
            Value,
        },
        Label, Operation,
    },
    r#type::{CompositeType, PointerSize, ScalarType, Type},
    version::Version,
};
use rspirv::{
    dr::{Instruction, Module, Operand},
    spirv::{
        Decoration, ExecutionMode as SpirvExecutionMode, FunctionControl, LoopControl,
        MemoryAccess, Op, SelectionControl,
    },
};
use spirv::{Capability, StorageClass};
use std::{
    cmp::Ordering,
    collections::HashMap,
    ops::{Deref, DerefMut},
    rc::Rc,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Constant {
    U32(u32),
    U64(u64),
    F32(u32),
    F64(u64),
    Bool(bool),
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

    pub fn constant_true(&mut self, result_type: rspirv::spirv::Word) -> rspirv::spirv::Word {
        *self
            .constants
            .entry((result_type, Constant::Bool(true)))
            .or_insert_with(|| self.inner.constant_true(result_type))
    }

    pub fn constant_false(&mut self, result_type: rspirv::spirv::Word) -> rspirv::spirv::Word {
        *self
            .constants
            .entry((result_type, Constant::Bool(false)))
            .or_insert_with(|| self.inner.constant_false(result_type))
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

impl<'a> ModuleBuilder<'a> {
    pub fn translate(mut self) -> Result<Builder> {
        let mut builder = Builder::new();
        builder.set_version(self.version.major, self.version.minor);

        // Memory model
        builder.memory_model(self.addressing_model, self.memory_model);

        // TODO entry points

        // TODO debug info

        // TODO anotations

        // Globals
        for global in self.global_variables.iter() {
            let _ = global.translate(&self, None, &mut builder)?;
        }

        // Function declarations
        for function in self.built_functions.iter() {
            function.function_id.set(Some(builder.id()));
        }

        // Hidden globals
        for global in self.hidden_global_variables.iter() {
            let _ = global.translate(&self, None, &mut builder)?;
        }

        // Function bodies
        for function in self.built_functions.iter() {
            function.translate(&self, &mut builder)?;
        }

        // Capabilities
        for capability in builder
            .module_ref()
            .all_inst_iter()
            .flat_map(|x| x.class.capabilities)
        {
            self.capabilities.require_mut(*capability)?;
        }

        for capability in self.capabilities.iter() {
            builder.capability(*capability)
        }

        // Extensions
        for extension in self.extensions.iter() {
            builder.extension(extension.to_string())
        }

        return Ok(builder);
    }
}

impl<'a> FunctionBuilder<'a> {
    pub fn translate(&self, module: &ModuleBuilder, builder: &mut Builder) -> Result<()> {
        let return_type = match &self.return_type {
            Some(ty) => ty.clone().translate(module, Some(self), builder)?,
            None => builder.type_void(),
        };
        let parameters = self
            .parameters
            .iter()
            .cloned()
            .map(|x| {
                x.ty(module)
                    .and_then(|x| x.translate(module, Some(self), builder))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let function_type = builder.type_function(return_type, parameters);

        // Initialize outside variables
        for var in self.outside_vars.iter() {
            let _ = var.translate(module, Some(self), builder)?;
        }

        // Create entry point
        if let Some(ref entry_point) = self.entry_point {
            let function_id = self.function_id.get().ok_or_else(Error::unexpected)?;
            let interface = entry_point
                .interface
                .iter()
                .map(|x| x.translate(module, Some(self), builder))
                .collect::<Result<Vec<_>>>()?;

            builder.entry_point(
                entry_point.execution_model,
                function_id,
                entry_point.name,
                interface,
            );

            // Add execution mode
            if let Some(ref exec_mode) = entry_point.execution_mode {
                let (execution_mode, params) = match exec_mode {
                    ExecutionMode::Invocations(x) => (SpirvExecutionMode::Invocations, vec![*x]),
                    ExecutionMode::PixelCenterInteger => {
                        (SpirvExecutionMode::PixelCenterInteger, Vec::new())
                    }
                    ExecutionMode::OriginUpperLeft => {
                        (SpirvExecutionMode::OriginUpperLeft, Vec::new())
                    }
                    ExecutionMode::OriginLowerLeft => {
                        (SpirvExecutionMode::OriginLowerLeft, Vec::new())
                    }
                    ExecutionMode::LocalSize(x, y, z) => {
                        (SpirvExecutionMode::LocalSize, vec![*x, *y, *z])
                    }
                    ExecutionMode::LocalSizeHint(x, y, z) => {
                        (SpirvExecutionMode::LocalSizeHint, vec![*x, *y, *z])
                    }
                };
                builder.execution_mode(function_id, execution_mode, params)
            }
        }

        builder.begin_function(
            return_type,
            self.function_id.get(),
            FunctionControl::NONE,
            function_type,
        )?;

        // Initialize function parameters
        for param in self.parameters.iter() {
            let _ = param.translate(module, Some(self), builder)?;
        }

        builder.begin_block(None)?;

        // Initialize
        for init in self.variable_initializers.iter() {
            let _ = init.translate(module, Some(self), builder)?;
        }

        // Translate anchors
        for anchor in self.anchors.iter() {
            let _ = anchor.translate(module, Some(self), builder)?;
        }

        builder.end_function()?;
        builder.select_block(None)?;

        return Ok(());
    }
}

pub trait Translation {
    fn translate(
        self,
        module: &ModuleBuilder,
        function: Option<&FunctionBuilder>,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word>;
}

/* TYPES */
impl Translation for ScalarType {
    fn translate(
        self,
        _: &ModuleBuilder,
        _: Option<&FunctionBuilder>,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        return Ok(match self {
            ScalarType::I32 => builder.type_int(32, 0),
            ScalarType::I64 => builder.type_int(64, 0),
            ScalarType::F32 => builder.type_float(32),
            ScalarType::F64 => builder.type_float(64),
            ScalarType::Bool => builder.type_bool(),
        });
    }
}

impl Translation for CompositeType {
    fn translate(
        self,
        module: &ModuleBuilder,
        function: Option<&FunctionBuilder>,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        match self {
            CompositeType::Vector(elem, component_count) => {
                let component_type = elem.translate(module, function, builder)?;
                Ok(builder.type_vector(component_type, component_count))
            }
        }
    }
}

impl Translation for Type {
    fn translate(
        self,
        module: &ModuleBuilder,
        function: Option<&FunctionBuilder>,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        match self {
            Type::Pointer {
                size,
                storage_class,
                pointee,
            } => {
                let pointee_type = pointee.clone().translate(module, function, builder)?;
                let is_structured = matches!(
                    storage_class,
                    StorageClass::Uniform
                        | StorageClass::StorageBuffer
                        | StorageClass::PhysicalStorageBuffer
                );

                // Fat (RuntimeArray)
                let pointee_type = match size {
                    PointerSize::Skinny => pointee_type,
                    PointerSize::Fat => {
                        let align = pointee
                            .comptime_byte_size(module)
                            .ok_or_else(Error::unexpected)?;

                        let n = builder.module_ref().types_global_values.len();
                        let runtime_array_type = builder.type_runtime_array(pointee_type);

                        if n != builder.module_ref().types_global_values.len() {
                            builder.decorate(
                                runtime_array_type,
                                Decoration::ArrayStride,
                                Some(Operand::LiteralInt32(align)),
                            );
                        }

                        runtime_array_type
                    }
                };

                // Structured
                let pointee_type = match is_structured {
                    false => pointee_type,
                    true => {
                        let n = builder.module_ref().types_global_values.len();
                        let structure_type = builder.type_struct([pointee_type]);

                        if n != builder.module_ref().types_global_values.len() {
                            builder.member_decorate(
                                structure_type,
                                0,
                                Decoration::Offset,
                                Some(Operand::LiteralInt32(0)),
                            );

                            let block = match module.version.cmp(&Version::V1_3) {
                                Ordering::Greater | Ordering::Equal => Decoration::Block,
                                _ => Decoration::BufferBlock,
                            };
                            builder.decorate(structure_type, block, None);
                        }

                        structure_type
                    }
                };

                Ok(builder.type_pointer(None, storage_class, pointee_type))
            }
            Type::Scalar(x) => x.translate(module, function, builder),
            Type::Composite(x) => x.translate(module, function, builder),
        }
    }
}

/* OPERATIONS */
impl Translation for &GlobalVariable {
    fn translate(
        self,
        module: &ModuleBuilder,
        function: Option<&FunctionBuilder>,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        match self {
            GlobalVariable::Variable(var) => var.translate(module, function, builder),
            GlobalVariable::Constant(cnst) => cnst.translate(module, function, builder),
        }
    }
}

impl Translation for &Schrodinger {
    fn translate(
        self,
        module: &ModuleBuilder,
        function: Option<&FunctionBuilder>,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        match self.variable.get() {
            Some(var) => var.translate(module, function, builder),
            None => todo!(),
        }
    }
}

impl Translation for &Bool {
    fn translate(
        self,
        module: &ModuleBuilder,
        function: Option<&FunctionBuilder>,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        if let Some(res) = self.translation.get() {
            return Ok(res);
        }

        let result_type = builder.type_bool();
        let res = match &self.source {
            BoolSource::Constant(true) => Ok(builder.constant_true(result_type)),
            BoolSource::Constant(false) => Ok(builder.constant_false(result_type)),

            BoolSource::FromInteger(int) => {
                let operand_1 = int.translate(module, function, builder)?;
                let zero = match int.kind(module)? {
                    IntegerKind::Short => {
                        let int_type = builder.type_int(32, 0);
                        builder.constant_u32(int_type, 0)
                    }
                    IntegerKind::Long => {
                        let int_type = builder.type_int(64, 0);
                        builder.constant_u64(int_type, 0)
                    }
                };
                builder.i_not_equal(result_type, None, operand_1, zero)
            }

            BoolSource::Select {
                selector,
                true_value,
                false_value,
            } => {
                let object_1 = true_value.translate(module, function, builder)?;
                let object_2 = false_value.translate(module, function, builder)?;
                let condition = selector.translate(module, function, builder)?;
                builder.select(result_type, None, condition, object_1, object_2)
            }

            BoolSource::IntEquality { kind, op1, op2 } => {
                let operand_1 = op1.translate(module, function, builder)?;
                let operand_2 = op2.translate(module, function, builder)?;
                match kind {
                    Equality::Eq => builder.i_equal(result_type, None, operand_1, operand_2),
                    Equality::Ne => builder.i_not_equal(result_type, None, operand_1, operand_2),
                }
            }

            BoolSource::FloatEquality { kind, op1, op2 } => {
                let operand_1 = op1.translate(module, function, builder)?;
                let operand_2 = op2.translate(module, function, builder)?;
                match kind {
                    Equality::Eq => builder.f_ord_equal(result_type, None, operand_1, operand_2),
                    Equality::Ne => {
                        builder.f_unord_not_equal(result_type, None, operand_1, operand_2)
                    }
                }
            }

            BoolSource::IntComparison {
                kind,
                signed,
                op1,
                op2,
            } => {
                let operand_1 = op1.translate(module, function, builder)?;
                let operand_2 = op2.translate(module, function, builder)?;
                match (*signed, kind) {
                    (true, Comparison::Ge) => {
                        builder.s_greater_than_equal(result_type, None, operand_1, operand_2)
                    }
                    (true, Comparison::Gt) => {
                        builder.s_greater_than(result_type, None, operand_1, operand_2)
                    }
                    (true, Comparison::Le) => {
                        builder.s_less_than_equal(result_type, None, operand_1, operand_2)
                    }
                    (true, Comparison::Lt) => {
                        builder.s_less_than(result_type, None, operand_1, operand_2)
                    }
                    (false, Comparison::Ge) => {
                        builder.u_greater_than_equal(result_type, None, operand_1, operand_2)
                    }
                    (false, Comparison::Gt) => {
                        builder.u_greater_than(result_type, None, operand_1, operand_2)
                    }
                    (false, Comparison::Le) => {
                        builder.u_less_than_equal(result_type, None, operand_1, operand_2)
                    }
                    (false, Comparison::Lt) => {
                        builder.u_less_than(result_type, None, operand_1, operand_2)
                    }
                }
            }

            BoolSource::FloatComparison { kind, op1, op2 } => {
                let operand_1 = op1.translate(module, function, builder)?;
                let operand_2 = op2.translate(module, function, builder)?;
                match kind {
                    Comparison::Le => {
                        builder.f_ord_less_than_equal(result_type, None, operand_1, operand_2)
                    }
                    Comparison::Lt => {
                        builder.f_ord_less_than(result_type, None, operand_1, operand_2)
                    }
                    Comparison::Gt => {
                        builder.f_ord_greater_than(result_type, None, operand_1, operand_2)
                    }
                    Comparison::Ge => {
                        builder.f_ord_greater_than_equal(result_type, None, operand_1, operand_2)
                    }
                }
            }

            BoolSource::Negated(x) => {
                let operand = x.translate(module, function, builder)?;
                builder.logical_not(result_type, None, operand)
            }

            BoolSource::Loaded {
                pointer,
                log2_alignment,
            } => {
                let pointee = &pointer.pointee;
                let storage_class = pointer.storage_class;
                let pointer = translate_to_skinny(pointer, module, function, builder)?;

                let (memory_access, additional_params) = additional_access_info(*log2_alignment);
                builder.load(result_type, None, pointer, memory_access, additional_params)
            }
        }?;

        self.translation.set(Some(res));
        return Ok(res);
    }
}

impl Translation for &Integer {
    fn translate(
        self,
        module: &ModuleBuilder,
        function: Option<&FunctionBuilder>,
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
                let structure = structured_array.translate(module, function, builder)?;
                builder.array_length(result_type, None, structure, 0)
            }

            IntegerSource::Constant(IntConstantSource::Short(x)) => {
                Ok(builder.constant_u32(result_type, *x))
            }

            IntegerSource::Constant(IntConstantSource::Long(x)) => {
                Ok(builder.constant_u64(result_type, *x))
            }

            IntegerSource::Select {
                selector,
                true_value,
                false_value,
            } => {
                let object_1 = true_value.translate(module, function, builder)?;
                let object_2 = false_value.translate(module, function, builder)?;
                let condition = selector.translate(module, function, builder)?;
                builder.select(result_type, None, condition, object_1, object_2)
            }

            IntegerSource::Conversion(
                IntConversionSource::FromLong(value)
                | IntConversionSource::FromShort {
                    signed: false,
                    value,
                },
            ) => {
                let unsigned_value = value.translate(module, function, builder)?;
                builder.u_convert(result_type, None, unsigned_value)
            }

            IntegerSource::Conversion(IntConversionSource::FromShort {
                signed: true,
                value,
            }) => {
                let unsigned_value = value.translate(module, function, builder)?;
                builder.s_convert(result_type, None, unsigned_value)
            }

            IntegerSource::Conversion(IntConversionSource::FromPointer(pointer)) => {
                let pointer = pointer.translate(module, function, builder)?;
                builder.convert_ptr_to_u(result_type, None, pointer)
            }

            IntegerSource::Conversion(IntConversionSource::FromBool(value, kind)) => {
                let (zero, one) = match kind {
                    IntegerKind::Short => (
                        builder.constant_u32(result_type, 0),
                        builder.constant_u32(result_type, 1),
                    ),
                    IntegerKind::Long => (
                        builder.constant_u64(result_type, 0),
                        builder.constant_u64(result_type, 1),
                    ),
                };

                let condition = value.translate(module, function, builder)?;
                builder.select(result_type, None, condition, one, zero)
            }

            IntegerSource::Loaded {
                pointer,
                log2_alignment,
            } => {
                let pointee = &pointer.pointee;
                let storage_class = pointer.storage_class;
                let pointer = translate_to_skinny(pointer, module, function, builder)?;

                let (memory_access, additional_params) = additional_access_info(*log2_alignment);
                builder.load(result_type, None, pointer, memory_access, additional_params)
            }

            IntegerSource::Extracted { vector, index } => {
                let composite = vector.translate(module, function, builder)?;
                match index.get_constant_value()? {
                    Some(IntConstantSource::Short(x)) => {
                        builder.composite_extract(result_type, None, composite, Some(x))
                    }
                    None => {
                        let index = index.translate(module, function, builder)?;
                        builder.vector_extract_dynamic(result_type, None, composite, index)
                    }
                    _ => todo!(),
                }
            }

            IntegerSource::FunctionCall {
                function_id, args, ..
            } => {
                let function_id = function_id.get().ok_or_else(Error::unexpected)?;
                let args = args
                    .iter()
                    .map(|x| x.translate(module, function, builder))
                    .collect::<Result<Vec<_>, _>>()?;

                builder.function_call(result_type, None, function_id, args)
            }

            IntegerSource::Unary { source, op1 } => {
                let operand = op1.translate(module, function, builder)?;
                match source {
                    IntUnarySource::Not => builder.not(result_type, None, operand),
                    IntUnarySource::Negate => builder.s_negate(result_type, None, operand),
                    IntUnarySource::BitCount => builder.bit_count(result_type, None, operand),
                    IntUnarySource::LeadingZeros => 'brk: {
                        for is in module.extended_is.iter() {
                            match is.kind {
                                ExtendedSet::OpenCL => {
                                    let extension_set = is.translate(module, function, builder)?;
                                    break 'brk builder.ext_inst(
                                        result_type,
                                        None,
                                        extension_set,
                                        OpenCLInstr::Clz as u32,
                                        Some(Operand::IdRef(operand)),
                                    );
                                }
                                _ => continue,
                            }
                        }

                        module
                            .capabilities
                            .require(Capability::IntegerFunctions2INTEL)?;
                        builder.u_count_leading_zeros_intel(result_type, None, operand)
                    }

                    IntUnarySource::TrainlingZeros => 'brk: {
                        for is in module.extended_is.iter() {
                            match is.kind {
                                ExtendedSet::OpenCL => {
                                    let extension_set = is.translate(module, function, builder)?;
                                    break 'brk builder.ext_inst(
                                        result_type,
                                        None,
                                        extension_set,
                                        OpenCLInstr::Ctz as u32,
                                        Some(Operand::IdRef(operand)),
                                    );
                                }
                                _ => continue,
                            }
                        }

                        module
                            .capabilities
                            .require(Capability::IntegerFunctions2INTEL)?;
                        builder.u_count_trailing_zeros_intel(result_type, None, operand)
                    }
                }
            }

            IntegerSource::Binary { source, op1, op2 } => {
                let operand_1 = op1.translate(module, function, builder)?;
                let operand_2 = op2.translate(module, function, builder)?;
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
                    IntBinarySource::Rotl => 'brk: {
                        for is in module.extended_is.iter() {
                            match is.kind {
                                ExtendedSet::OpenCL => {
                                    let extension_set = is.translate(module, function, builder)?;
                                    break 'brk builder.ext_inst(
                                        result_type,
                                        None,
                                        extension_set,
                                        OpenCLInstr::Rotate as u32,
                                        [Operand::IdRef(operand_1), Operand::IdRef(operand_2)],
                                    );
                                }
                                _ => continue,
                            }
                        }
                        todo!()
                    }
                    IntBinarySource::Rotr => 'brk: {
                        todo!()
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
        function: Option<&FunctionBuilder>,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        if let Some(res) = self.translation.get() {
            return Ok(res);
        }

        let kind = self.kind()?;
        let result_bits = match kind {
            FloatKind::Single => 32,
            FloatKind::Double => 64,
        };
        let result_type = builder.type_float(result_bits);
        let integer_type = builder.type_int(result_bits, 0);

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
                let float_value = value.translate(module, function, builder)?;
                builder.f_convert(result_type, None, float_value)
            }
            FloatSource::Select {
                selector,
                true_value,
                false_value,
            } => {
                let object_1 = true_value.translate(module, function, builder)?;
                let object_2 = false_value.translate(module, function, builder)?;
                let condition = selector.translate(module, function, builder)?;
                builder.select(result_type, None, condition, object_1, object_2)
            }
            FloatSource::Loaded {
                pointer,
                log2_alignment,
            } => {
                let pointee = &pointer.pointee;
                let storage_class = pointer.storage_class;
                let pointer = translate_to_skinny(pointer, module, function, builder)?;

                let (memory_access, additional_params) = additional_access_info(*log2_alignment);
                builder.load(result_type, None, pointer, memory_access, additional_params)
            }
            FloatSource::Extracted { vector, index } => {
                let composite = vector.translate(module, function, builder)?;
                match index.get_constant_value()? {
                    Some(IntConstantSource::Short(x)) => {
                        builder.composite_extract(result_type, None, composite, Some(x))
                    }
                    None => {
                        let index = index.translate(module, function, builder)?;
                        builder.vector_extract_dynamic(result_type, None, composite, index)
                    }
                    _ => todo!(),
                }
            }
            FloatSource::FunctionCall {
                function_id, args, ..
            } => {
                let function_id = function_id.get().ok_or_else(Error::unexpected)?;
                let args = args
                    .iter()
                    .map(|x| x.translate(module, function, builder))
                    .collect::<Result<Vec<_>, _>>()?;

                builder.function_call(result_type, None, function_id, args)
            }
            FloatSource::Unary { source, op1 } => {
                let operand = op1.translate(module, function, builder)?;
                match source {
                    FloatUnarySource::Neg => builder.f_negate(result_type, None, operand),
                    FloatUnarySource::Abs => 'brk: {
                        for is in module.extended_is.iter() {
                            match is.kind {
                                ExtendedSet::GLSL450 => {
                                    let extension_set = is.translate(module, function, builder)?;
                                    break 'brk builder.ext_inst(
                                        result_type,
                                        None,
                                        extension_set,
                                        GLSLInstr::Fabs as u32,
                                        Some(Operand::IdRef(operand)),
                                    );
                                }
                                ExtendedSet::OpenCL => {
                                    let extension_set = is.translate(module, function, builder)?;
                                    break 'brk builder.ext_inst(
                                        result_type,
                                        None,
                                        extension_set,
                                        OpenCLInstr::Fabs as u32,
                                        Some(Operand::IdRef(operand)),
                                    );
                                }
                            }
                        }

                        let mask = match result_bits {
                            32 => Integer::new_constant_u32((1 << 31) - 1)
                                .translate(module, function, builder)?,
                            64 => Integer::new_constant_u64((1 << 63) - 1)
                                .translate(module, function, builder)?,
                            _ => return Err(Error::unexpected()),
                        };
                        let integer = builder.bitcast(integer_type, None, operand)?;
                        let masked = builder.bitwise_and(integer_type, None, integer, mask)?;
                        break 'brk builder.bitcast(result_type, None, masked);
                    }
                    FloatUnarySource::Ceil => 'brk: {
                        for is in module.extended_is.iter() {
                            match is.kind {
                                ExtendedSet::GLSL450 => {
                                    let extension_set = is.translate(module, function, builder)?;
                                    break 'brk builder.ext_inst(
                                        result_type,
                                        None,
                                        extension_set,
                                        GLSLInstr::Ceil as u32,
                                        Some(Operand::IdRef(operand)),
                                    );
                                }
                                ExtendedSet::OpenCL => {
                                    let extension_set = is.translate(module, function, builder)?;
                                    break 'brk builder.ext_inst(
                                        result_type,
                                        None,
                                        extension_set,
                                        OpenCLInstr::Ceil as u32,
                                        Some(Operand::IdRef(operand)),
                                    );
                                }
                            }
                        }
                        return Err(Error::msg(
                            "Ceil rounding is not supported on this platform",
                        ));
                    }
                    FloatUnarySource::Floor => 'brk: {
                        for is in module.extended_is.iter() {
                            match is.kind {
                                ExtendedSet::GLSL450 => {
                                    let extension_set = is.translate(module, function, builder)?;
                                    break 'brk builder.ext_inst(
                                        result_type,
                                        None,
                                        extension_set,
                                        GLSLInstr::Floor as u32,
                                        Some(Operand::IdRef(operand)),
                                    );
                                }
                                ExtendedSet::OpenCL => {
                                    let extension_set = is.translate(module, function, builder)?;
                                    break 'brk builder.ext_inst(
                                        result_type,
                                        None,
                                        extension_set,
                                        OpenCLInstr::Floor as u32,
                                        Some(Operand::IdRef(operand)),
                                    );
                                }
                            }
                        }
                        return Err(Error::msg(
                            "Floor rounding is not supported on this platform",
                        ));
                    }
                    FloatUnarySource::Trunc => 'brk: {
                        for is in module.extended_is.iter() {
                            match is.kind {
                                ExtendedSet::GLSL450 => {
                                    let extension_set = is.translate(module, function, builder)?;
                                    break 'brk builder.ext_inst(
                                        result_type,
                                        None,
                                        extension_set,
                                        GLSLInstr::Trunc as u32,
                                        Some(Operand::IdRef(operand)),
                                    );
                                }
                                ExtendedSet::OpenCL => {
                                    let extension_set = is.translate(module, function, builder)?;
                                    break 'brk builder.ext_inst(
                                        result_type,
                                        None,
                                        extension_set,
                                        OpenCLInstr::Trunc as u32,
                                        Some(Operand::IdRef(operand)),
                                    );
                                }
                            }
                        }
                        return Err(Error::msg(
                            "Truncating rounding is not supported on this platform",
                        ));
                    }
                    FloatUnarySource::Nearest => 'brk: {
                        for is in module.extended_is.iter() {
                            match is.kind {
                                ExtendedSet::GLSL450 => {
                                    let extension_set = is.translate(module, function, builder)?;
                                    break 'brk builder.ext_inst(
                                        result_type,
                                        None,
                                        extension_set,
                                        GLSLInstr::RoundEven as u32,
                                        Some(Operand::IdRef(operand)),
                                    );
                                }
                                ExtendedSet::OpenCL => {
                                    let extension_set = is.translate(module, function, builder)?;
                                    break 'brk builder.ext_inst(
                                        result_type,
                                        None,
                                        extension_set,
                                        OpenCLInstr::Rint as u32,
                                        Some(Operand::IdRef(operand)),
                                    );
                                }
                            }
                        }
                        return Err(Error::msg(
                            "Rounding (ties to even) rounding is not supported on this platform",
                        ));
                    }
                    FloatUnarySource::Sqrt => 'brk: {
                        for is in module.extended_is.iter() {
                            match is.kind {
                                ExtendedSet::GLSL450 => {
                                    let extension_set = is.translate(module, function, builder)?;
                                    break 'brk builder.ext_inst(
                                        result_type,
                                        None,
                                        extension_set,
                                        GLSLInstr::Sqrt as u32,
                                        Some(Operand::IdRef(operand)),
                                    );
                                }
                                ExtendedSet::OpenCL => {
                                    let extension_set = is.translate(module, function, builder)?;
                                    break 'brk builder.ext_inst(
                                        result_type,
                                        None,
                                        extension_set,
                                        OpenCLInstr::Sqrt as u32,
                                        Some(Operand::IdRef(operand)),
                                    );
                                }
                            }
                        }
                        return Err(Error::msg("Square root is not supported on this platform"));
                    }
                }
            }
            FloatSource::Binary { source, op1, op2 } => {
                let operand_1 = op1.translate(module, function, builder)?;
                let operand_2 = op2.translate(module, function, builder)?;
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
                    FloatBinarySource::Copysign => 'brk: {
                        for is in module.extended_is.iter() {
                            match is.kind {
                                ExtendedSet::OpenCL => {
                                    let extension_set = is.translate(module, function, builder)?;
                                    break 'brk builder.ext_inst(
                                        result_type,
                                        None,
                                        extension_set,
                                        OpenCLInstr::Copysign as u32,
                                        [Operand::IdRef(operand_1), Operand::IdRef(operand_2)],
                                    );
                                }
                                _ => continue,
                            }
                        }

                        todo!()
                    }
                    FloatBinarySource::Min => {
                        const F32_NAN_ODDS: u32 = (1u32 << f32::MANTISSA_DIGITS) - 2;
                        const F32_OTHER_ODDS: u32 = u32::MAX - F32_NAN_ODDS;
                        const F64_NAN_ODDS: u64 = (1u64 << f64::MANTISSA_DIGITS) - 2;
                        const F64_OTHER_ODDS: u64 = u64::MAX - F64_NAN_ODDS;

                        let (nan, nan_odds, other_odds) = match result_bits {
                            32 => (
                                Float::new_constant_f32(f32::NAN)
                                    .translate(module, function, builder)?,
                                F32_NAN_ODDS,
                                F32_OTHER_ODDS,
                            ),
                            64 => (
                                Float::new_constant_f64(f64::NAN)
                                    .translate(module, function, builder)?,
                                (F64_NAN_ODDS >> 32) as u32,
                                (F64_OTHER_ODDS >> 32) as u32,
                            ),
                            _ => return Err(Error::unexpected()),
                        };

                        let boolean = builder.type_bool();
                        let is_nan_1 = builder.is_nan(boolean, None, operand_1)?;
                        let is_nan_2 = builder.is_nan(boolean, None, operand_2)?;
                        let is_nan = builder.logical_or(boolean, None, is_nan_1, is_nan_2)?;

                        let true_label = builder.id();
                        let false_label = builder.id();
                        let merge_label = builder.id();

                        let result = Rc::new(Pointer::new_variable(
                            PointerSize::Skinny,
                            StorageClass::Function,
                            kind,
                            None,
                            Vec::new(),
                        ))
                        .translate(module, function, builder)?;

                        let current_block = builder.selected_block();
                        builder.selection_merge(merge_label, SelectionControl::FLATTEN)?;
                        builder.select_block(current_block)?;
                        builder.branch_conditional(
                            is_nan,
                            true_label,
                            false_label,
                            [nan_odds, other_odds],
                        )?;

                        builder.begin_block(Some(true_label))?;
                        builder.store(result, nan, None, None)?;
                        builder.branch(merge_label)?;

                        builder.begin_block(Some(false_label))?;
                        let fast_min = fast_fmin(
                            boolean,
                            result_type,
                            module,
                            function,
                            builder,
                            operand_1,
                            operand_2,
                        )?;
                        builder.store(result, fast_min, None, None)?;
                        builder.branch(merge_label)?;

                        builder.begin_block(Some(merge_label))?;
                        builder.load(result_type, None, result, None, None)
                    }
                    FloatBinarySource::Max => {
                        const F32_NAN_ODDS: u32 = (1u32 << f32::MANTISSA_DIGITS) - 2;
                        const F32_OTHER_ODDS: u32 = u32::MAX - F32_NAN_ODDS;
                        const F64_NAN_ODDS: u64 = (1u64 << f64::MANTISSA_DIGITS) - 2;
                        const F64_OTHER_ODDS: u64 = u64::MAX - F64_NAN_ODDS;

                        let (nan, nan_odds, other_odds) = match result_bits {
                            32 => (
                                Float::new_constant_f32(f32::NAN)
                                    .translate(module, function, builder)?,
                                F32_NAN_ODDS,
                                F32_OTHER_ODDS,
                            ),
                            64 => (
                                Float::new_constant_f64(f64::NAN)
                                    .translate(module, function, builder)?,
                                (F64_NAN_ODDS >> 32) as u32,
                                (F64_OTHER_ODDS >> 32) as u32,
                            ),
                            _ => return Err(Error::unexpected()),
                        };

                        let boolean = builder.type_bool();
                        let is_nan_1 = builder.is_nan(boolean, None, operand_1)?;
                        let is_nan_2 = builder.is_nan(boolean, None, operand_2)?;
                        let is_nan = builder.logical_or(boolean, None, is_nan_1, is_nan_2)?;

                        let true_label = builder.id();
                        let false_label = builder.id();
                        let merge_label = builder.id();

                        let current_block = builder.selected_block();
                        builder.selection_merge(merge_label, SelectionControl::FLATTEN)?;
                        builder.select_block(current_block)?;
                        builder.branch_conditional(
                            is_nan,
                            true_label,
                            false_label,
                            [nan_odds, other_odds],
                        )?;

                        let result = Rc::new(Pointer::new_variable(
                            PointerSize::Skinny,
                            StorageClass::Function,
                            kind,
                            None,
                            Vec::new(),
                        ))
                        .translate(module, function, builder)?;

                        builder.begin_block(Some(true_label))?;
                        builder.store(result, nan, None, None)?;
                        builder.branch(merge_label)?;

                        builder.begin_block(Some(false_label))?;
                        let fast_max = fast_fmax(
                            boolean,
                            result_type,
                            module,
                            function,
                            builder,
                            operand_1,
                            operand_2,
                        )?;
                        builder.store(result, fast_max, None, None)?;
                        builder.branch(merge_label)?;

                        builder.begin_block(Some(merge_label))?;
                        builder.load(result_type, None, result, None, None)
                    }
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
        function: Option<&FunctionBuilder>,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        let translation = match &self.kind {
            PointerKind::Skinny { translation } => translation.get(),
            PointerKind::Fat { translation, .. } => translation.get(),
        };

        if let Some(res) = translation {
            return Ok(res);
        }

        let pointer_type = Type::pointer(
            self.kind.to_pointer_size(),
            self.storage_class,
            self.pointee.clone(),
        )
        .translate(module, function, builder)?;

        let res = match &self.source {
            PointerSource::FunctionParam => builder.function_parameter(pointer_type),

            PointerSource::Casted { prev } => {
                let prev = prev.translate(module, function, builder)?;
                builder.bitcast(pointer_type, None, prev)
            }

            PointerSource::FromInteger(value) => {
                let integer_value = value.translate(module, function, builder)?;
                builder.convert_u_to_ptr(pointer_type, None, integer_value)
            }

            PointerSource::Select {
                selector,
                true_value,
                false_value,
            } => {
                let object_1 = true_value.translate(module, function, builder)?;
                let object_2 = false_value.translate(module, function, builder)?;
                let condition = selector.translate(module, function, builder)?;
                builder.select(pointer_type, None, condition, object_1, object_2)
            }

            PointerSource::Loaded {
                pointer,
                log2_alignment,
            } => {
                let pointee = &pointer.pointee;
                let storage_class = pointer.storage_class;

                let pointer = pointer.translate(module, function, builder)?;
                let (memory_access, additional_params) = additional_access_info(*log2_alignment);
                builder.load(
                    pointer_type,
                    None,
                    pointer,
                    memory_access,
                    additional_params,
                )
            }

            PointerSource::Variable { init, decorators } => {
                let initializer = init
                    .as_ref()
                    .map(|x| x.translate(module, function, builder))
                    .transpose()?;

                let mut operands = vec![Operand::StorageClass(self.storage_class)];
                if let Some(val) = initializer {
                    operands.push(Operand::IdRef(val));
                }

                let id = builder.id();
                let variable =
                    Instruction::new(Op::Variable, Some(pointer_type), Some(id), operands);

                match builder.selected_block().is_some() {
                    true => builder.insert_into_block(rspirv::dr::InsertPoint::Begin, variable)?,
                    false => builder.module_mut().types_global_values.push(variable),
                }

                decorators.iter().for_each(|x| x.translate(id, builder));
                Ok(id)
            }
        }?;

        match &self.kind {
            PointerKind::Skinny { translation } => translation.set(Some(res)),
            PointerKind::Fat { translation, .. } => translation.set(Some(res)),
        };
        return Ok(res);
    }
}

impl Translation for &Vector {
    fn translate(
        self,
        module: &ModuleBuilder,
        function: Option<&FunctionBuilder>,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        if let Some(res) = self.translation.get() {
            return Ok(res);
        }

        let result_type = self.vector_type().translate(module, function, builder)?;
        let res = match &self.source {
            VectorSource::Loaded {
                pointer,
                log2_alignment,
            } => {
                let pointee = &pointer.pointee;
                let storage_class = pointer.storage_class;
                let pointer = translate_to_skinny(pointer, module, function, builder)?;

                let (memory_access, additional_params) = additional_access_info(*log2_alignment);
                builder.load(result_type, None, pointer, memory_access, additional_params)
            }
            VectorSource::Select {
                selector,
                true_value,
                false_value,
            } => {
                let object_1 = true_value.translate(module, function, builder)?;
                let object_2 = false_value.translate(module, function, builder)?;
                let condition = selector.translate(module, function, builder)?;
                builder.select(result_type, None, condition, object_1, object_2)
            }
        }?;

        self.translation.set(Some(res));
        return Ok(res);
    }
}

impl Translation for &Value {
    fn translate(
        self,
        module: &ModuleBuilder,
        function: Option<&FunctionBuilder>,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        match self {
            Value::Integer(x) => x.translate(module, function, builder),
            Value::Float(x) => x.translate(module, function, builder),
            Value::Pointer(x) => x.translate(module, function, builder),
            Value::Vector(x) => x.translate(module, function, builder),
            Value::Bool(x) => x.translate(module, function, builder),
        }
    }
}

impl Translation for &Label {
    fn translate(
        self,
        _: &ModuleBuilder,
        _: Option<&FunctionBuilder>,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        if let Some(res) = self.translation.get() {
            return Ok(res);
        }

        let id = builder.id();
        self.translation.set(Some(id));
        return Ok(id);
    }
}

impl Translation for &Operation {
    fn translate(
        self,
        module: &ModuleBuilder,
        function: Option<&FunctionBuilder>,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        match self {
            Operation::Value(x) => {
                return x.translate(module, function, builder);
            }

            Operation::Label(x) => {
                let label = x.translate(module, function, builder)?;
                builder.insert_into_block(
                    rspirv::dr::InsertPoint::End,
                    rspirv::dr::Instruction::new(Op::Label, None, Some(label), Vec::new()),
                )?;
                return Ok(label);
            }

            Operation::Branch { label } => {
                let function =
                    function.ok_or_else(|| Error::msg("Branches must be inside a function"))?;

                let selected = builder.selected_block();
                let target_label = label.translate(module, Some(function), builder)?;

                // TODO control flow

                let res = builder.branch(target_label);
                builder.select_block(selected)?;
                res
            }

            Operation::BranchConditional {
                condition,
                true_label,
                false_label,
            } => {
                let function =
                    function.ok_or_else(|| Error::msg("Branches must be inside a function"))?;

                let current_block = function.block_of(self).ok_or_else(Error::unexpected)?;
                let true_block = function.block(true_label).last();
                let false_block = function.block(false_label).last();

                // control flow
                let (merge_block, continue_target) = match (true_block, false_block) {
                    // Both blocks end up branching to the same block.
                    // This is probably a structured if
                    (
                        Some(Operation::Branch { label }),
                        Some(Operation::Branch { label: label1 }),
                    ) if Rc::ptr_eq(label, label1) => (Some(label.clone()), None),

                    // True block ends up branching to the current label.
                    // True block is probably the continue target, leaving the false block as the "exit" branch
                    (Some(Operation::Branch { label }), _) if label == current_block => {
                        (Some(false_label.clone()), Some(true_label.clone()))
                    }

                    // True block ends up branching to the false label.
                    (Some(Operation::Branch { label }), _) if label == false_label => {
                        (Some(false_label.clone()), None)
                    }

                    (_, Some(Operation::Branch { label })) if label == true_label => {
                        (Some(true_label.clone()), None)
                    }

                    // False block ends up branching to the current label.
                    // False block is probably the continue target, leaving the true block as the "exit" branch
                    (_, Some(Operation::Branch { label })) if label == current_block => {
                        (Some(true_label.clone()), Some(false_label.clone()))
                    }
                    _ => (None, None),
                };

                let selected = builder.selected_block();
                let true_label = true_label.translate(module, Some(function), builder)?;
                let false_label = false_label.translate(module, Some(function), builder)?;

                match (&condition.source, continue_target) {
                    (BoolSource::FromInteger(int), None) => {
                        let selector = int.translate(module, Some(function), builder)?;
                        let zero = match int.kind(module)? {
                            IntegerKind::Short => Operand::LiteralInt32(0),
                            IntegerKind::Long => Operand::LiteralInt64(0),
                        };

                        // control flow
                        if let Some(merge_block) = merge_block {
                            let merge_block =
                                merge_block.translate(module, Some(function), builder)?;
                            let block = builder.selected_block();
                            builder.selection_merge(merge_block, SelectionControl::NONE)?;
                            builder.select_block(block)?;
                        }

                        builder.switch(selector, true_label, Some((zero, false_label)))
                    }

                    (_, continue_target) => {
                        let condition = condition.translate(module, Some(function), builder)?;

                        // control flow
                        let block = builder.selected_block();
                        match (merge_block, continue_target) {
                            (Some(merge_block), None) => {
                                let merge_block =
                                    merge_block.translate(module, Some(function), builder)?;
                                builder.selection_merge(merge_block, SelectionControl::NONE)?;
                            }

                            (Some(merge_block), Some(continue_target)) => {
                                let merge_block =
                                    merge_block.translate(module, Some(function), builder)?;

                                let continue_target =
                                    continue_target.translate(module, Some(function), builder)?;

                                builder.loop_merge(
                                    merge_block,
                                    continue_target,
                                    LoopControl::NONE,
                                    None,
                                )?;
                            }

                            (None, None) => {}

                            _ => return Err(Error::unexpected()),
                        }
                        builder.select_block(block)?;

                        builder.branch_conditional(condition, true_label, false_label, None)
                    }
                }?;

                builder.select_block(selected)
            }

            Operation::Store {
                target: pointer,
                value,
                log2_alignment,
            } => {
                let pointer = translate_to_skinny(pointer, module, function, builder)?;
                let object = value.translate(module, function, builder)?;
                let (memory_access, additional_params) = additional_access_info(*log2_alignment);

                builder.store(pointer, object, memory_access, additional_params)
            }

            Operation::Copy {
                src,
                src_log2_alignment,
                dst,
                dst_log2_alignment,
            } => {
                let src = src.translate(module, function, builder)?;
                let dst = dst.translate(module, function, builder)?;
                let (
                    mut memory_access_1,
                    mut memory_access_2,
                    additional_params_1,
                    additional_params_2,
                );

                if module.version >= Version::V1_4 {
                    (memory_access_2, additional_params_2) = src_log2_alignment
                        .map(|align| (MemoryAccess::ALIGNED, Operand::LiteralInt32(1 << align)))
                        .unzip();

                    (memory_access_1, additional_params_1) = dst_log2_alignment
                        .map(|align| (MemoryAccess::ALIGNED, Operand::LiteralInt32(1 << align)))
                        .unzip();

                    memory_access_1.get_or_insert(MemoryAccess::NONE);
                    memory_access_2.get_or_insert(MemoryAccess::NONE);
                } else {
                    (memory_access_2, additional_params_2) = (None, None);
                    (memory_access_1, additional_params_1) =
                        match (src_log2_alignment, dst_log2_alignment) {
                            (Some(src_log2_alignment), Some(dst_log2_alignment))
                                if src_log2_alignment == dst_log2_alignment =>
                            {
                                (
                                    Some(MemoryAccess::ALIGNED),
                                    Some(Operand::LiteralInt32(1 << src_log2_alignment)),
                                )
                            }
                            _ => (None, None),
                        }
                }

                let additional_params = additional_params_2.into_iter().chain(additional_params_1);
                builder.copy_memory(
                    dst,
                    src,
                    memory_access_2,
                    memory_access_1,
                    additional_params,
                )
            }

            Operation::FunctionCall { function_id, args } => {
                let function_id = function_id.get().ok_or_else(Error::unexpected)?;
                let args = args
                    .iter()
                    .map(|x| x.translate(module, function, builder))
                    .collect::<Result<Vec<_>, _>>()?;

                let void = builder.type_void();
                builder.function_call(void, None, function_id, args)?;
                Ok(())
            }

            Operation::Nop => {
                let selected = builder.selected_block();
                builder.nop()?;
                builder.select_block(selected)
            }

            Operation::Unreachable => {
                let selected = builder.selected_block();
                builder.unreachable()?;
                builder.select_block(selected)
            }

            Operation::Return { value: Some(value) } => {
                let selected = builder.selected_block();
                let value = value.clone().translate(module, function, builder)?;
                let res = builder.ret_value(value);
                builder.select_block(selected)?;
                res
            }

            Operation::Return { value: None } => {
                let selected = builder.selected_block();
                builder.ret()?;
                builder.select_block(selected)
            }
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
    cfg_if::cfg_if! {
        if #[cfg(feature = "naga")] {
            (None, None)
        } else {
            log2_alignment
                .map(|align| (MemoryAccess::ALIGNED, Operand::LiteralInt32(1 << align)))
                .unzip()
        }
    }
}

fn translate_to_skinny(
    pointer: &Rc<Pointer>,
    module: &ModuleBuilder,
    function: Option<&FunctionBuilder>,
    builder: &mut Builder,
) -> Result<spirv::Word> {
    let pointer_word = pointer.translate(module, function, builder)?;
    let mut indexes = Vec::with_capacity(2);

    let result_type = match pointer.is_structured() {
        true => {
            let zero = Rc::new(Integer::new_constant_usize(0, module))
                .translate(module, function, builder)?;
            indexes.push(zero);

            let pointee_type = pointer
                .pointee
                .clone()
                .translate(module, function, builder)?;
            builder.type_pointer(None, pointer.storage_class, pointee_type)
        }

        false => Type::pointer(
            PointerSize::Skinny,
            pointer.storage_class,
            pointer.pointee.clone(),
        )
        .translate(module, function, builder)?,
    };

    if pointer.is_fat() {
        let stride = pointer
            .pointee
            .comptime_byte_size(module)
            .ok_or_else(Error::unexpected)?;

        let stride = Rc::new(Integer::new_constant_usize(stride, module));
        let offset = pointer
            .byte_offset()
            .unwrap_or_else(|| Rc::new(Integer::new_constant_usize(0, module)))
            .u_div(stride, module)?
            .translate(module, function, builder)?;

        indexes.push(offset);
    }

    return match indexes.is_empty() {
        true => Ok(pointer_word),
        false => builder
            .access_chain(result_type, None, pointer_word, indexes)
            .map_err(Into::into),
    };
}

fn fast_fmin(
    boolean: spirv::Word,
    result_type: spirv::Word,
    module: &ModuleBuilder,
    function: Option<&FunctionBuilder>,
    builder: &mut Builder,
    operand_1: spirv::Word,
    operand_2: spirv::Word,
) -> Result<spirv::Word> {
    for is in module.extended_is.iter() {
        match is.kind {
            ExtendedSet::GLSL450 => {
                let extension_set = is.translate(module, function, builder)?;
                return Ok(builder.ext_inst(
                    result_type,
                    None,
                    extension_set,
                    GLSLInstr::Fmin as u32,
                    [Operand::IdRef(operand_1), Operand::IdRef(operand_2)],
                )?);
            }
            ExtendedSet::OpenCL => {
                let extension_set = is.translate(module, function, builder)?;
                return Ok(builder.ext_inst(
                    result_type,
                    None,
                    extension_set,
                    OpenCLInstr::Fmin as u32,
                    [Operand::IdRef(operand_1), Operand::IdRef(operand_2)],
                )?);
            }
        }
    }

    let condition = builder.f_unord_less_than_equal(boolean, None, operand_1, operand_2)?;
    builder
        .select(result_type, None, condition, operand_1, operand_2)
        .map_err(Into::into)
}

fn fast_fmax(
    boolean: spirv::Word,
    result_type: spirv::Word,
    module: &ModuleBuilder,
    function: Option<&FunctionBuilder>,
    builder: &mut Builder,
    operand_1: spirv::Word,
    operand_2: spirv::Word,
) -> Result<spirv::Word> {
    for is in module.extended_is.iter() {
        match is.kind {
            ExtendedSet::GLSL450 => {
                let extension_set = is.translate(module, function, builder)?;
                return Ok(builder.ext_inst(
                    result_type,
                    None,
                    extension_set,
                    GLSLInstr::Fmax as u32,
                    [Operand::IdRef(operand_1), Operand::IdRef(operand_2)],
                )?);
            }
            ExtendedSet::OpenCL => {
                let extension_set = is.translate(module, function, builder)?;
                return Ok(builder.ext_inst(
                    result_type,
                    None,
                    extension_set,
                    OpenCLInstr::Fmax as u32,
                    [Operand::IdRef(operand_1), Operand::IdRef(operand_2)],
                )?);
            }
        }
    }

    let condition = builder.f_unord_greater_than_equal(boolean, None, operand_1, operand_2)?;
    builder
        .select(result_type, None, condition, operand_1, operand_2)
        .map_err(Into::into)
}
