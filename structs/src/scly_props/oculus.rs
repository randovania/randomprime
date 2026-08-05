use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr, RoArray};

use crate::{scly_props::structs::*, SclyPropertyData};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct Oculus<'r> {
    /// 15 everywhere except PAL and NTSC-J, which append one trailing float.
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub patterned_info: PatternedInfo,
    pub actor_params: ActorParameters,

    #[auto_struct(init = (if prop_count == 15 { 164 } else { 168 }, ()))]
    pub dont_care: RoArray<'r, u8>,
}

use crate::impl_actor_scannable_parameters;
impl SclyPropertyData for Oculus<'_> {
    const OBJECT_TYPE: u8 = 0x6F;
    impl_actor_scannable_parameters!(actor_params);
}
