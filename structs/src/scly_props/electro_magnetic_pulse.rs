use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct ElectroMagneticPulse<'r> {
    #[auto_struct(expect = 12)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub dont_care: GenericArray<u8, U57>,
}

impl SclyPropertyData for ElectroMagneticPulse<'_> {
    const OBJECT_TYPE: u8 = 0x4A;
}
