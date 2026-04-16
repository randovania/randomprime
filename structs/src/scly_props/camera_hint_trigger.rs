use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::U3, CStr};

use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct CameraHintTrigger<'r> {
    #[auto_struct(expect = 7)]
    prop_count: u32,

    pub name: CStr<'r>,
    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,
    pub active: u8,
    pub deactivate_on_enter: u8,
    pub deactivate_on_exit: u8,
}

use crate::{impl_position, impl_rotation, impl_scale};
impl SclyPropertyData for CameraHintTrigger<'_> {
    const OBJECT_TYPE: u8 = 0x73;

    impl_position!();
    impl_rotation!();
    impl_scale!();
}
