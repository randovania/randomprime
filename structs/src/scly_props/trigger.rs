use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{scly_props::structs::DamageInfo, SclyPropertyData};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Trigger<'r> {
    #[auto_struct(expect = 9)]
    prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,
    pub damage_info: DamageInfo,
    pub force: GenericArray<f32, U3>,
    pub flags: u32,
    pub active: u8,
    pub deactivate_on_enter: u8,
    pub deactivate_on_exit: u8,
}

use crate::{impl_position, impl_scale};
impl SclyPropertyData for Trigger<'_> {
    const OBJECT_TYPE: u8 = 0x04;
    impl_position!();
    impl_scale!();

    const SUPPORTS_DAMAGE_INFOS: bool = true;

    fn impl_get_damage_infos(&self) -> Vec<DamageInfo> {
        vec![self.damage_info]
    }

    fn impl_set_damage_infos(&mut self, x: Vec<DamageInfo>) {
        self.damage_info = x[0];
    }
}
