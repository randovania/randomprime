use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct WallCrawlerSwarm<'r> {
    #[auto_struct(expect = 39)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub dont_care: GenericArray<u8, U454>,
}

impl SclyPropertyData for WallCrawlerSwarm<'_> {
    const OBJECT_TYPE: u8 = 0x5A;
}
