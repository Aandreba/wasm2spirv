use crate::{
    ast::function::{ExecutionMode, FunctionConfig, Parameter, ParameterKind},
    config::{AddressingModel, CapabilityModel, Config, ExtensionModel, WasmFeatures},
    error::{Error, Result},
    r#type::{CompositeType, ScalarType, Type},
    version::{TargetPlatform, Version},
    Str,
};
use num_traits::cast::FromPrimitive;
use spirv::{Capability, ExecutionModel, MemoryModel, StorageClass};
use std::hash::Hash;
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
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        let vec = Vec::<u8>::deserialize_from(reader)?;
        String::from_utf8(vec).map_err(|e| Error::msg(e.to_string()))
    }
}

impl<'a> BinaryDeserialize for Str<'a> {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        Box::<str>::deserialize_from(reader).map(Self::Owned)
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
            1 => Ok(Version::deserialize_from(reader).map(Self::Universal)?),
            _ => Err(Error::msg("Unknown kind")),
        };
    }
}

impl BinaryDeserialize for AddressingModel {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        Self::try_from(reader.read_u16()?).map_err(Error::custom)
    }
}

impl BinaryDeserialize for MemoryModel {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        Self::from_u32(reader.read_u32()?).ok_or_else(|| Error::msg("Unknown memory model"))
    }
}

impl BinaryDeserialize for Capability {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        Self::from_u32(reader.read_u32()?).ok_or_else(|| Error::msg("Unknown capability"))
    }
}

impl BinaryDeserialize for ExecutionModel {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        Self::from_u32(reader.read_u32()?).ok_or_else(|| Error::msg("Unknown execution model"))
    }
}

impl BinaryDeserialize for StorageClass {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        Self::from_u32(reader.read_u32()?).ok_or_else(|| Error::msg("Unknown storage class"))
    }
}

impl BinaryDeserialize for CapabilityModel {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        let kind = reader.read_u8()?;
        let capabilities = Vec::<Capability>::deserialize_from(reader)?;

        return Ok(match kind {
            0 => CapabilityModel::Static(capabilities.into_boxed_slice()),
            1 => CapabilityModel::Dynamic(capabilities),
            _ => return Err(Error::msg("Unkown capability model")),
        });
    }
}

impl BinaryDeserialize for ExtensionModel {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        let kind = reader.read_u8()?;
        let extensions = Vec::<Str>::deserialize_from(reader)?;

        return Ok(match kind {
            0 => ExtensionModel::Static(extensions.into_boxed_slice()),
            1 => ExtensionModel::Dynamic(extensions),
            _ => return Err(Error::msg("Unkown extension model")),
        });
    }
}

impl BinaryDeserialize for ExecutionMode {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        return Ok(match reader.read_u16()? {
            0 => reader.read_u32().map(ExecutionMode::Invocations)?,
            1 => ExecutionMode::PixelCenterInteger,
            2 => ExecutionMode::OriginUpperLeft,
            3 => ExecutionMode::OriginLowerLeft,
            4 => {
                ExecutionMode::LocalSize(reader.read_u32()?, reader.read_u32()?, reader.read_u32()?)
            }
            5 => ExecutionMode::LocalSizeHint(
                reader.read_u32()?,
                reader.read_u32()?,
                reader.read_u32()?,
            ),
            _ => return Err(Error::msg("Unknown execution mode")),
        });
    }
}

impl BinaryDeserialize for ScalarType {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        Self::try_from(reader.read_u16()?).map_err(Error::custom)
    }
}

impl BinaryDeserialize for CompositeType {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        return match reader.read_u16()? {
            0 => ScalarType::deserialize_from(reader).map(CompositeType::Structured),
            1 => ScalarType::deserialize_from(reader).map(CompositeType::StructuredArray),
            2 => {
                let elem = ScalarType::deserialize_from(reader)?;
                let count = reader.read_u32()?;
                Ok(CompositeType::Vector(elem, count))
            }
            _ => return Err(Error::msg("Unknown composite type")),
        };
    }
}

impl BinaryDeserialize for Type {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        return match reader.read_u16()? {
            0 => {
                let storage_class = StorageClass::deserialize_from(reader)?;
                let pointee = Type::deserialize_from(reader)?;
                Ok(Type::Pointer(storage_class, Box::new(pointee)))
            }
            1 => ScalarType::deserialize_from(reader).map(Self::Scalar),
            2 => CompositeType::deserialize_from(reader).map(Self::Composite),
            _ => return Err(Error::msg("Unkown type")),
        };
    }
}

impl BinaryDeserialize for ParameterKind {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        return Ok(match reader.read_u16()? {
            0 => Self::FunctionParameter,
            1 => Self::Input,
            2 => Self::Output,
            3 => Self::DescriptorSet {
                storage_class: StorageClass::deserialize_from(reader)?,
                set: reader.read_u32()?,
                binding: reader.read_u32()?,
            },
            _ => return Err(Error::msg("Unknown parameter kind")),
        });
    }
}

impl BinaryDeserialize for Parameter {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        return Ok(Self {
            ty: BinaryDeserialize::deserialize_from(reader)?,
            kind: BinaryDeserialize::deserialize_from(reader)?,
            is_extern_pointer: BinaryDeserialize::deserialize_from(reader)?,
        });
    }
}

impl BinaryDeserialize for FunctionConfig {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        return Ok(Self {
            execution_model: BinaryDeserialize::deserialize_from(reader)?,
            execution_mode: BinaryDeserialize::deserialize_from(reader)?,
            params: BinaryDeserialize::deserialize_from(reader)?,
        });
    }
}

impl BinaryDeserialize for Config {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        return Ok(Self {
            platform: BinaryDeserialize::deserialize_from(reader)?,
            features: BinaryDeserialize::deserialize_from(reader)?,
            addressing_model: BinaryDeserialize::deserialize_from(reader)?,
            memory_model: BinaryDeserialize::deserialize_from(reader)?,
            capabilities: BinaryDeserialize::deserialize_from(reader)?,
            extensions: BinaryDeserialize::deserialize_from(reader)?,
            functions: BinaryDeserialize::deserialize_from(reader)?,
        });
    }
}

// BLANKETS
impl<K, V> BinaryDeserialize for HashMap<K, V>
where
    K: Eq + Hash + BinaryDeserialize,
    V: BinaryDeserialize,
{
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        let len = reader.read_u32()? as usize;

        let mut result = Self::with_capacity(len);
        for _ in 0..len {
            let key = K::deserialize_from(reader)?;
            let value = V::deserialize_from(reader)?;
            result.insert(key, value);
        }

        return Ok(result);
    }
}

impl<K: Eq + Ord + BinaryDeserialize, V: BinaryDeserialize> BinaryDeserialize for BTreeMap<K, V> {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        let len = reader.read_u32()? as usize;

        let mut result = Self::new();
        for _ in 0..len {
            let key = K::deserialize_from(reader)?;
            let value = V::deserialize_from(reader)?;
            result.insert(key, value);
        }

        return Ok(result);
    }
}

impl<K: Eq + BinaryDeserialize, V: BinaryDeserialize> BinaryDeserialize for VecMap<K, V> {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        let len = reader.read_u32()? as usize;

        let mut result = Self::with_capacity(len);
        for _ in 0..len {
            let key = K::deserialize_from(reader)?;
            let value = V::deserialize_from(reader)?;
            result.insert(key, value);
        }

        return Ok(result);
    }
}

impl<T: BinaryDeserialize> BinaryDeserialize for Option<T> {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        return match reader.read_u8()? {
            0 => Ok(None),
            1 => T::deserialize_from(reader).map(Some),
            _ => Err(Error::msg("Unknown option")),
        };
    }
}

impl<T: BinaryDeserialize> BinaryDeserialize for Vec<T> {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        let len = reader.read_u32()? as usize;

        let mut result: Vec<T> = Self::with_capacity(len);
        for _ in 0..len {
            let value = T::deserialize_from(reader)?;
            result.push(value);
        }

        return Ok(result);
    }
}

impl<T: BinaryDeserialize> BinaryDeserialize for Box<T> {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        T::deserialize_from(reader).map(Box::new)
    }
}

impl<T: BinaryDeserialize> BinaryDeserialize for Box<[T]> {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        Vec::<T>::deserialize_from(reader).map(Vec::into_boxed_slice)
    }
}

impl BinaryDeserialize for Box<str> {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        String::deserialize_from(reader).map(String::into_boxed_str)
    }
}

impl<T: ?Sized + BinaryDeserialize> BinaryDeserialize for Rc<T> {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        T::deserialize_from(reader).map(Rc::new)
    }
}

impl<T: BinaryDeserialize> BinaryDeserialize for Rc<[T]> {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        Vec::<T>::deserialize_from(reader).map(Rc::from)
    }
}

impl BinaryDeserialize for Rc<str> {
    fn deserialize_from<R: ?Sized + std::io::Read>(reader: &mut R) -> Result<Self> {
        String::deserialize_from(reader).map(Rc::from)
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
