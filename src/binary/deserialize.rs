use crate::{
    ast::function::{ExecutionMode, FunctionConfig, Parameter, ParameterKind},
    config::{AddressingModel, CapabilityModel, Config, ExtensionModel, WasmFeatures},
    error::{Error, Result},
    r#type::{CompositeType, ScalarType, Type},
    version::{TargetPlatform, Version},
    Str,
};
use spirv::{Capability, ExecutionModel, MemoryModel, StorageClass};
use std::{
    collections::{BTreeMap, HashMap},
    mem::{size_of, MaybeUninit},
    rc::Rc,
};
use vector_mapp::vec::VecMap;

pub trait BinaryDeserialize: Sized {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self>;
}

/* SERIALIZE */
impl BinaryDeserialize for String {
    #[inline]
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        let vec = Vec::<u8>::deserialize_from(reader)?;
        todo!()
    }
}

impl<'a> BinaryDeserialize for Str<'a> {
    #[inline]
    fn deserialize_from<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        str::deserialize_from(self, writer)
    }
}

impl BinaryDeserialize for bool {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        return Ok(match reader.read_u8()? {
            0 => false,
            1 => true,
            _ => return Err(Error::msg("Non-valid boolean value")),
        });
    }
}

impl BinaryDeserialize for u8 {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        return reader.read_u8().map_err(Into::into);
    }
}

impl BinaryDeserialize for u32 {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        return reader.read_u32().map_err(Into::into);
    }
}

impl BinaryDeserialize for WasmFeatures {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        reader
            .read_u64()
            .map(Self::from_integer)
            .map_err(Into::into)
    }
}

impl BinaryDeserialize for Version {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        let major = reader.read_u8()?;
        let minor = reader.read_u8()?;
        return Ok(Self::new(major, minor));
    }
}

impl BinaryDeserialize for TargetPlatform {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        let kind = reader.read_u16()?;
        return match kind {
            0 => Ok(Version::deserialize_from(reader).map(Self::Vulkan)?),
            _ => Err(Error::msg("Unknown kind")),
        };
    }
}

impl BinaryDeserialize for AddressingModel {
    fn deserialize_from<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u16(*self as u16)?;
        Ok(())
    }
}

impl BinaryDeserialize for MemoryModel {
    fn deserialize_from<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u32(*self as u32)?;
        Ok(())
    }
}

impl BinaryDeserialize for Capability {
    fn deserialize_from<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u32(*self as u32)?;
        Ok(())
    }
}

impl BinaryDeserialize for ExecutionModel {
    fn deserialize_from<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u32(*self as u32)?;
        Ok(())
    }
}

impl BinaryDeserialize for StorageClass {
    fn deserialize_from<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u32(*self as u32)?;
        Ok(())
    }
}

impl BinaryDeserialize for CapabilityModel {
    fn deserialize_from<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        match self {
            CapabilityModel::Static(data) => {
                writer.write_u8(0)?;
                data.deserialize_from(writer)?;
            }
            CapabilityModel::Dynamic(data) => {
                writer.write_u8(1)?;
                data.deserialize_from(writer)?;
            }
        };

        Ok(())
    }
}

impl BinaryDeserialize for ExtensionModel {
    fn deserialize_from<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        match self {
            ExtensionModel::Static(data) => {
                writer.write_u8(0)?;
                data.deserialize_from(writer)?;
            }
            ExtensionModel::Dynamic(data) => {
                writer.write_u8(1)?;
                data.deserialize_from(writer)?;
            }
        };

        Ok(())
    }
}

impl BinaryDeserialize for ExecutionMode {
    fn deserialize_from<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        match self {
            ExecutionMode::Invocations(x) => {
                writer.write_u16(0)?;
                writer.write_u32(*x)?;
            }
            ExecutionMode::PixelCenterInteger => writer.write_u16(1)?,
            ExecutionMode::OriginUpperLeft => writer.write_u16(2)?,
            ExecutionMode::OriginLowerLeft => writer.write_u16(3)?,
            ExecutionMode::LocalSize(x, y, z) => {
                writer.write_u16(4)?;
                writer.write_u32(*x)?;
                writer.write_u32(*y)?;
                writer.write_u32(*z)?;
            }
            ExecutionMode::LocalSizeHint(x, y, z) => {
                writer.write_u16(5)?;
                writer.write_u32(*x)?;
                writer.write_u32(*y)?;
                writer.write_u32(*z)?;
            }
        }

        Ok(())
    }
}

impl BinaryDeserialize for ScalarType {
    fn deserialize_from<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u16(*self as u16)?;
        Ok(())
    }
}

impl BinaryDeserialize for CompositeType {
    fn deserialize_from<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        match self {
            CompositeType::Structured(elem) => {
                writer.write_u16(0)?;
                elem.deserialize_from(writer)?;
            }
            CompositeType::StructuredArray(elem) => {
                writer.write_u16(1)?;
                elem.deserialize_from(writer)?;
            }
            CompositeType::Vector(elem, count) => {
                writer.write_u16(2)?;
                elem.deserialize_from(writer)?;
                writer.write_u32(*count)?;
            }
        };

        Ok(())
    }
}

impl BinaryDeserialize for Type {
    fn deserialize_from<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        match self {
            Type::Pointer(sc, pointee) => {
                writer.write_u16(0)?;
                sc.deserialize_from(writer)?;
                pointee.deserialize_from(writer)?;
            }
            Type::Scalar(x) => {
                writer.write_u16(1)?;
                x.deserialize_from(writer)?;
            }
            Type::Composite(x) => {
                writer.write_u16(2)?;
                x.deserialize_from(writer)?;
            }
        };

        Ok(())
    }
}

impl BinaryDeserialize for ParameterKind {
    fn deserialize_from<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        match self {
            ParameterKind::FunctionParameter => writer.write_u16(0)?,
            ParameterKind::Input => writer.write_u16(1)?,
            ParameterKind::Output => writer.write_u16(2)?,
            ParameterKind::DescriptorSet {
                storage_class,
                set,
                binding,
            } => {
                writer.write_u16(3)?;
                storage_class.deserialize_from(writer)?;
                writer.write_u32(*set)?;
                writer.write_u32(*binding)?;
            }
        };

        Ok(())
    }
}

impl BinaryDeserialize for Parameter {
    fn deserialize_from<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        self.ty.deserialize_from(writer)?;
        self.kind.deserialize_from(writer)?;
        self.is_extern_pointer.deserialize_from(writer)?;
        Ok(())
    }
}

impl BinaryDeserialize for FunctionConfig {
    fn deserialize_from<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        self.entry_point_exec_model.deserialize_from(writer)?;
        self.exec_mode.deserialize_from(writer)?;
        self.params.deserialize_from(writer)?;
        Ok(())
    }
}

impl BinaryDeserialize for Config {
    fn deserialize_from<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        self.platform.deserialize_from(writer)?;
        self.version.deserialize_from(writer)?;
        self.features.deserialize_from(writer)?;
        self.addressing_model.deserialize_from(writer)?;
        self.memory_model.deserialize_from(writer)?;
        self.capabilities.deserialize_from(writer)?;
        self.extensions.deserialize_from(writer)?;
        self.functions.deserialize_from(writer)?;
        Ok(())
    }
}

// BLANKETS
impl<K: BinaryDeserialize, V: BinaryDeserialize> BinaryDeserialize for HashMap<K, V> {
    fn deserialize_from<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        let len = u32::try_from(self.len()).map_err(|e| Error::msg(e.to_string()))?;

        writer.write_u32(len)?;
        for (key, value) in self {
            key.deserialize_from(writer)?;
            value.deserialize_from(writer)?;
        }

        Ok(())
    }
}

impl<K: BinaryDeserialize, V: BinaryDeserialize> BinaryDeserialize for BTreeMap<K, V> {
    fn deserialize_from<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        let len = u32::try_from(self.len()).map_err(|e| Error::msg(e.to_string()))?;

        writer.write_u32(len)?;
        for (key, value) in self {
            key.deserialize_from(writer)?;
            value.deserialize_from(writer)?;
        }

        Ok(())
    }
}

impl<K: BinaryDeserialize, V: BinaryDeserialize> BinaryDeserialize for VecMap<K, V> {
    fn deserialize_from<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        let len = u32::try_from(self.len()).map_err(|e| Error::msg(e.to_string()))?;

        writer.write_u32(len)?;
        for (key, value) in self {
            key.deserialize_from(writer)?;
            value.deserialize_from(writer)?;
        }

        Ok(())
    }
}

impl<T: BinaryDeserialize> BinaryDeserialize for Option<T> {
    fn deserialize_from<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        match self {
            Some(val) => {
                writer.write_u8(1)?;
                val.deserialize_from(writer)?;
            }
            None => writer.write_u8(0)?,
        };

        Ok(())
    }
}

impl<T: BinaryDeserialize> BinaryDeserialize for Vec<T> {
    #[inline]
    fn deserialize_from<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        <[T]>::deserialize_from(self, writer)
    }
}

impl<T: ?Sized + BinaryDeserialize> BinaryDeserialize for Box<T> {
    #[inline]
    fn deserialize_from<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        T::deserialize_from(self, writer)
    }
}

impl<T: ?Sized + BinaryDeserialize> BinaryDeserialize for Rc<T> {
    #[inline]
    fn deserialize_from<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        T::deserialize_from(self, writer)
    }
}

pub(crate) trait ReadLe: std::io::Read {
    fn read_i8(&mut self) -> std::io::Result<i8> {
        let mut res: MaybeUninit<[u8; size_of::<i8>()]> = MaybeUninit::uninit();
        unsafe {
            self.read_exact(&mut *res.as_mut_ptr())?;
            return Ok(i8::from_le_bytes(res.assume_init()));
        }
    }

    fn read_u8(&mut self) -> std::io::Result<u8> {
        let mut res: MaybeUninit<[u8; size_of::<u8>()]> = MaybeUninit::uninit();
        unsafe {
            self.read_exact(&mut *res.as_mut_ptr())?;
            return Ok(u8::from_le_bytes(res.assume_init()));
        }
    }

    fn read_i16(&mut self) -> std::io::Result<i16> {
        let mut res: MaybeUninit<[u8; size_of::<i16>()]> = MaybeUninit::uninit();
        unsafe {
            self.read_exact(&mut *res.as_mut_ptr())?;
            return Ok(i16::from_le_bytes(res.assume_init()));
        }
    }

    fn read_u16(&mut self) -> std::io::Result<u16> {
        let mut res: MaybeUninit<[u8; size_of::<u16>()]> = MaybeUninit::uninit();
        unsafe {
            self.read_exact(&mut *res.as_mut_ptr())?;
            return Ok(u16::from_le_bytes(res.assume_init()));
        }
    }

    fn read_i32(&mut self) -> std::io::Result<i32> {
        let mut res: MaybeUninit<[u8; size_of::<i32>()]> = MaybeUninit::uninit();
        unsafe {
            self.read_exact(&mut *res.as_mut_ptr())?;
            return Ok(i32::from_le_bytes(res.assume_init()));
        }
    }

    fn read_u32(&mut self) -> std::io::Result<u32> {
        let mut res: MaybeUninit<[u8; size_of::<u32>()]> = MaybeUninit::uninit();
        unsafe {
            self.read_exact(&mut *res.as_mut_ptr())?;
            return Ok(u32::from_le_bytes(res.assume_init()));
        }
    }

    fn read_i64(&mut self) -> std::io::Result<i64> {
        let mut res: MaybeUninit<[u8; size_of::<i64>()]> = MaybeUninit::uninit();
        unsafe {
            self.read_exact(&mut *res.as_mut_ptr())?;
            return Ok(i64::from_le_bytes(res.assume_init()));
        }
    }

    fn read_u64(&mut self) -> std::io::Result<u64> {
        let mut res: MaybeUninit<[u8; size_of::<u64>()]> = MaybeUninit::uninit();
        unsafe {
            self.read_exact(&mut *res.as_mut_ptr())?;
            return Ok(u64::from_le_bytes(res.assume_init()));
        }
    }

    fn read_i128(&mut self) -> std::io::Result<i128> {
        let mut res: MaybeUninit<[u8; size_of::<i128>()]> = MaybeUninit::uninit();
        unsafe {
            self.read_exact(&mut *res.as_mut_ptr())?;
            return Ok(i128::from_le_bytes(res.assume_init()));
        }
    }

    fn read_u128(&mut self) -> std::io::Result<u128> {
        let mut res: MaybeUninit<[u8; size_of::<u128>()]> = MaybeUninit::uninit();
        unsafe {
            self.read_exact(&mut *res.as_mut_ptr())?;
            return Ok(u128::from_le_bytes(res.assume_init()));
        }
    }

    fn read_f32(&mut self) -> std::io::Result<f32> {
        let mut res: MaybeUninit<[u8; size_of::<f32>()]> = MaybeUninit::uninit();
        unsafe {
            self.read_exact(&mut *res.as_mut_ptr())?;
            return Ok(f32::from_le_bytes(res.assume_init()));
        }
    }

    fn read_f64(&mut self) -> std::io::Result<f64> {
        let mut res: MaybeUninit<[u8; size_of::<f64>()]> = MaybeUninit::uninit();
        unsafe {
            self.read_exact(&mut *res.as_mut_ptr())?;
            return Ok(f64::from_le_bytes(res.assume_init()));
        }
    }
}

impl<T: ?Sized + std::io::Read> ReadLe for T {}
