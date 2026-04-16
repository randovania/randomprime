use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct NewCameraShaker<'r> {
    #[auto_struct(expect = 8)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub active: u8,

    pub unknown1: u32,
    pub unknown2: u8,

    pub duration: f32,
    pub sfx_dist: f32,

    pub shakers: GenericArray<NewCameraShakerComponent, U3>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct NewCameraShakerComponent {
    pub unknown1: u32,
    pub unknown2: u8,
    pub am: NewCameraShakePoint,
    pub fm: NewCameraShakePoint,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct NewCameraShakePoint {
    pub unknown1: u32,
    pub unknown2: u8,
    pub attack_time: f32,
    pub sustain_time: f32,
    pub duration: f32,
    pub magnitude: f32,
}

impl SclyPropertyData for NewCameraShaker<'_> {
    const OBJECT_TYPE: u8 = 0x89;
}
