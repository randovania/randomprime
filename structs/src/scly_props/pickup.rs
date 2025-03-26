use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{
    res_id::*,
    scly_props::structs::{ActorParameters, AncsProp},
    ResId, SclyPropertyData,
};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Pickup<'r> {
    #[auto_struct(expect = 18)]
    prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,
    pub hitbox: GenericArray<f32, U3>,
    pub scan_offset: GenericArray<f32, U3>,

    pub kind: u32,

    pub max_increase: i32,
    pub curr_increase: i32,

    pub drop_rate: f32,
    pub disappear_timer: f32,
    pub fade_in_timer: f32,

    pub cmdl: ResId<CMDL>,
    pub ancs: AncsProp,
    pub actor_params: ActorParameters,

    pub active: u8,
    pub spawn_delay: f32,
    pub part: ResId<PART>,
}

use crate::{impl_position, impl_rotation, impl_scale};
impl SclyPropertyData for Pickup<'_> {
    const OBJECT_TYPE: u8 = 0x11;

    impl_position!();
    impl_rotation!();
    impl_scale!();
}
