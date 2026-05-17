use std::io;

use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, RoArray, typenum::*, CStr, Readable, Reader, Writable};

#[auto_struct(Readable, Writable)]
#[derive(Clone, Debug)]
pub struct Atbl<'r> {
    
    #[auto_struct(derive = entries.len() as u32)]
    pub count: u32,
    #[auto_struct(init = (count as usize, ()))]
    pub entries: RoArray<'r, GenericArray<u32, U2>>,

    #[auto_struct(pad_align = 32)]
    _pad: (),
}
