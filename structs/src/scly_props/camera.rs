use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Camera<'r> {
    #[auto_struct(expect = 15)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub active: u8,
    pub shot_duration: f32,
    pub look_at_player: u8,
    pub out_of_player_eye: u8,
    pub into_player_eye: u8,
    pub draw_player: u8,
    pub disable_input: u8,
    pub unknown: u8,
    pub finish_cine_skip: u8,
    pub field_of_view: f32,
    pub check_failsafe: u8,
    pub disable_out_of_into: u8,
}

use crate::{impl_position, impl_rotation};
impl SclyPropertyData for Camera<'_> {
    const OBJECT_TYPE: u8 = 0x0C;

    impl_position!();
    impl_rotation!();
}
