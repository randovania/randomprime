use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{
    scly_props::structs::{ActorParameters, AncsProp},
    SclyPropertyData,
};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Door<'r> {
    #[auto_struct(expect = 14)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub ancs: AncsProp,
    pub actor_params: ActorParameters,

    pub scan_offset: GenericArray<f32, U3>,
    pub collision_size: GenericArray<f32, U3>,
    pub collision_offset: GenericArray<f32, U3>,

    pub active: u8,
    pub open: u8,
    pub projectiles_collide: u8,
    pub open_close_animation_len: f32,
    pub is_morphball_door: u8,
}

use crate::{impl_position, impl_rotation, impl_scale};
impl SclyPropertyData for Door<'_> {
    const OBJECT_TYPE: u8 = 0x03;

    impl_position!();
    impl_rotation!();
    impl_scale!();
}
