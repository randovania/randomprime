use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{scly_props::structs::ScannableParameters, SclyPropertyData};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct PointOfInterest<'r> {
    #[auto_struct(expect = 6)]
    prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub active: u8,
    pub scan_param: ScannableParameters,
    pub point_size: f32,
}

use crate::{impl_position, impl_rotation};
impl SclyPropertyData for PointOfInterest<'_> {
    const OBJECT_TYPE: u8 = 0x42;
    impl_position!();
    impl_rotation!();
}
