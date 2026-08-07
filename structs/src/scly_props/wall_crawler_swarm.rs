use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{scly_props::structs::*, SclyPropertyData};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct WallCrawlerSwarm<'r> {
    #[auto_struct(expect = 39)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,
    pub active: u8,
    pub actor_params: ActorParameters,

    pub dont_care: GenericArray<u8, U292>,
}

use crate::impl_actor_scannable_parameters;
impl SclyPropertyData for WallCrawlerSwarm<'_> {
    const OBJECT_TYPE: u8 = 0x5A;
    impl_actor_scannable_parameters!(actor_params);
}
