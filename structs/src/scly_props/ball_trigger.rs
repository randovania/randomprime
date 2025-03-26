use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::U3, CStr};

use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct BallTrigger<'r> {
    #[auto_struct(expect = 9)]
    prop_count: u32,

    pub name: CStr<'r>,
    pub position: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,
    pub active: u8,
    pub force: f32,
    pub min_angle: f32,
    pub max_distance: f32,
    pub force_angle: GenericArray<f32, U3>,
    pub stop_player: u8,
}

use crate::{impl_position, impl_scale};
impl SclyPropertyData for BallTrigger<'_> {
    const OBJECT_TYPE: u8 = 0x48;

    impl_position!();
    impl_scale!();
}
