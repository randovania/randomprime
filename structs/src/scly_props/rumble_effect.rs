use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr, LazyArray};

use crate::{impl_active, impl_position, SclyPropertyData};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct RumbleEffect<'r> {
    #[auto_struct(expect = 6)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub position: GenericArray<f32, U3>,
    pub active: u8,
    pub intensity: f32,
    pub flags: u32,

    #[auto_struct(derive = parameter_flags.len() as u32)]
    parameter_flag_count: u32,
    #[auto_struct(init = (parameter_flag_count as usize, ()))]
    pub parameter_flags: LazyArray<'r, u8>,
}

impl SclyPropertyData for RumbleEffect<'_> {
    const OBJECT_TYPE: u8 = 0x74;

    impl_active!();
    impl_position!();
}
