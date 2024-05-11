// Rexport these crates to make syncing version numbers less of a pain
pub use byteorder;
pub use generic_array;

pub mod reader;
pub mod writer;

pub mod array;
pub mod fixed_array;
pub mod iterator_array;
pub mod primitive_types;
pub mod read_only_array;

pub mod derivable_array_proxy;
pub mod lcow;
pub mod uncached;
pub mod with_read;

pub mod padding;

pub mod utf16_string;

pub use crate::{
    array::{LazyArray, LazyArrayIter},
    derivable_array_proxy::{Dap, DerivableFromIterator},
    fixed_array::FixedArray,
    generic_array::typenum,

    iterator_array::{IteratorArray, IteratorArrayIterator},
    lcow::LCow,

    // XXX There are > 5 items in these modules. Do I want to use * imports everywhere for
    //     consistency?
    padding::*,
    primitive_types::{CStr, CStrConversionExtension, FourCC},
    read_only_array::{RoArray, RoArrayIter},
    reader::{Readable, Reader},
    uncached::Uncached,
    utf16_string::*,
    with_read::WithRead,

    writer::Writable,
};
