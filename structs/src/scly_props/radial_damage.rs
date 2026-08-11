use crate::{impl_active, impl_position, scly_props::structs::DamageInfo, SclyPropertyData};
use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct RadialDamage<'r> {
    #[auto_struct(expect = 5)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub position: GenericArray<f32, U3>,
    pub active: u8,
    pub damage_info: DamageInfo,
    pub radius: f32,
}

impl SclyPropertyData for RadialDamage<'_> {
    const OBJECT_TYPE: u8 = 0x68;

    impl_active!();
    impl_position!();
}
