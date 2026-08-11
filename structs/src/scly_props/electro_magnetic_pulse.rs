use crate::{impl_active, impl_position, impl_rotation, res_id::*, ResId, SclyPropertyData};
use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct ElectroMagneticPulse<'r> {
    #[auto_struct(expect = 12)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub active: u8,
    pub unknown0: f32,
    pub unknown1: f32,
    pub unknown2: f32,
    pub unknown3: f32,
    pub unknown4: f32,
    pub unknown5: f32,
    pub unknown6: f32,
    pub particle: ResId<PART>,
}

impl SclyPropertyData for ElectroMagneticPulse<'_> {
    const OBJECT_TYPE: u8 = 0x4A;

    impl_active!();
    impl_position!();
    impl_rotation!();
}
