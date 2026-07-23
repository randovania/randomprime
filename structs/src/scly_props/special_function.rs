use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr, CStrConversionExtension};

use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct SpecialFunction<'r> {
    #[auto_struct(expect = 15)]
    prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,

    pub type_: u32,

    pub string_param: CStr<'r>,
    pub value_param: f32,
    pub value_param2: f32,
    pub value_param3: f32,

    pub layer_change_room_id: u32,
    pub layer_change_layer_id: u32,
    pub item_id: u32,

    pub active: u8,
    pub value_param4: f32,

    // "Used by SpinnerController"
    pub sound1: u32,
    pub sound2: u32,
    pub sound3: u32,
}

use crate::{impl_active, impl_position, impl_rotation};
impl SclyPropertyData for SpecialFunction<'_> {
    const OBJECT_TYPE: u8 = 0x3A;
    impl_active!();
    impl_position!();
    impl_rotation!();
}

impl<'r> SpecialFunction<'r> {
    pub fn layer_change_fn(name: CStr<'r>, room_id: u32, layer_num: u32) -> Self {
        SpecialFunction {
            name,
            position: [0., 0., 0.].into(),
            rotation: [0., 0., 0.].into(),
            type_: 16,
            string_param: b"\0".as_cstr(),
            value_param: 0.,
            value_param2: 0.,
            value_param3: 0.,
            layer_change_room_id: room_id,
            layer_change_layer_id: layer_num,
            item_id: 0,
            active: 1,
            value_param4: 0.,
            sound1: 0xFFFFFFFF,
            sound2: 0xFFFFFFFF,
            sound3: 0xFFFFFFFF,
        }
    }

    pub fn ice_trap_fn(name: CStr<'r>) -> Self {
        SpecialFunction {
            name,
            position: [0., 0., 0.].into(),
            rotation: [0., 0., 0.].into(),
            type_: 33,
            string_param: b"\0".as_cstr(),
            value_param: 0.,
            value_param2: 0.,
            value_param3: 0.,
            layer_change_room_id: 0,
            layer_change_layer_id: u32::MAX,
            item_id: 0,
            active: 1,
            value_param4: 0.,
            sound1: 0xFFFFFFFF,
            sound2: 0xFFFFFFFF,
            sound3: 0xFFFFFFFF,
        }
    }
}
