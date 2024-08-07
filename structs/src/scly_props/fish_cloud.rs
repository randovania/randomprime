use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{impl_position, impl_rotation, impl_scale, scly_props::structs::*, SclyPropertyData};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct FishCloud<'r> {
    #[auto_struct(expect = 36)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub active: u8,
    pub cmdl: u32,
    pub animation_params: AnimationParameters,
    pub num_boids: u32,
    pub speed: f32,

    pub dont_cares1: GenericArray<f32, U17>,
    pub done_care1: u8,
    pub dont_cares2: GenericArray<f32, U10>,
    pub done_care2: u8,
    pub done_care3: u8,
}

impl<'r> SclyPropertyData for FishCloud<'r> {
    const OBJECT_TYPE: u8 = 0x4F;

    impl_position!();
    impl_rotation!();
    impl_scale!();
}
