use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{
    scly_props::structs::{ActorParameters, AncsProp},
    SclyPropertyData,
};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct Door<'r> {
    #[auto_struct(expect = 14)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub animation_parameters: AncsProp,
    pub actor_parameters: ActorParameters,

    pub orbit_position: GenericArray<f32, U3>,
    pub collision_size: GenericArray<f32, U3>,
    pub collision_offset: GenericArray<f32, U3>,

    pub active: u8,
    pub open: u8,
    pub projectiles_collide: u8,
    pub animation_length: f32,
    pub is_morphball_door: u8,
}

use crate::{
    impl_active, impl_actor_scannable_parameters, impl_position, impl_rotation, impl_scale,
};
impl SclyPropertyData for Door<'_> {
    const OBJECT_TYPE: u8 = 0x03;

    impl_active!();
    impl_position!();
    impl_rotation!();
    impl_scale!();
    impl_actor_scannable_parameters!(actor_parameters);
}
