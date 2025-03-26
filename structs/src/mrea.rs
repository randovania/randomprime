use std::io;

use auto_struct_macros::auto_struct;
use reader_writer::{
    generic_array::GenericArray, typenum::*, IteratorArray, LCow, LazyArray, Readable, Reader,
    RoArray, RoArrayIter, Writable,
};

use crate::scly::Scly;

#[auto_struct(Readable, Writable)]
#[derive(Clone, Debug)]
pub struct Mrea<'r> {
    #[auto_struct(expect = 0xDEADBEEF)]
    magic: u32,

    #[auto_struct(expect = 0xF)]
    version: u32,

    pub area_transform: GenericArray<f32, U12>,
    pub world_model_count: u32,

    #[auto_struct(derive = sections.len() as u32)]
    sections_count: u32,

    pub world_geometry_section_idx: u32,
    pub scly_section_idx: u32,
    pub collision_section_idx: u32,
    pub unknown_section_idx: u32,
    pub lights_section_idx: u32,
    pub visibility_tree_section_idx: u32,
    pub path_section_idx: u32,
    pub area_octree_section_idx: u32,

    #[auto_struct(derive_from_iter = sections.iter()
            .map(&|i: LCow<MreaSection>| i.size() as u32))]
    #[auto_struct(init = (sections_count as usize, ()))]
    section_sizes: RoArray<'r, u32>,

    #[auto_struct(pad_align = 32)]
    _pad: (),

    #[auto_struct(init = section_sizes.iter())]
    pub sections: IteratorArray<'r, MreaSection<'r>, RoArrayIter<'r, u32>>,

    #[auto_struct(pad_align = 32)]
    _pad: (),
}

impl<'r> Mrea<'r> {
    pub fn scly_section<'s>(&'s self) -> LCow<'s, Scly<'r>> {
        let section = self
            .sections
            .iter()
            .nth(self.scly_section_idx as usize)
            .unwrap();
        match section {
            LCow::Owned(MreaSection::Unknown(ref reader)) => LCow::Owned(reader.clone().read(())),
            LCow::Borrowed(MreaSection::Unknown(ref reader)) => {
                LCow::Owned(reader.clone().read(()))
            }
            LCow::Owned(MreaSection::Scly(scly)) => LCow::Owned(scly),
            LCow::Borrowed(MreaSection::Scly(scly)) => LCow::Borrowed(scly),
            _ => unreachable!(),
        }
    }

    pub fn scly_section_mut(&mut self) -> &mut Scly<'r> {
        self.sections.as_mut_vec()[self.scly_section_idx as usize].convert_to_scly()
    }

    pub fn lights_section<'s>(&'s self) -> LCow<'s, Lights<'r>> {
        let section = self
            .sections
            .iter()
            .nth(self.lights_section_idx as usize)
            .unwrap();
        match section {
            LCow::Owned(MreaSection::Unknown(ref reader)) => LCow::Owned(reader.clone().read(())),
            LCow::Borrowed(MreaSection::Unknown(ref reader)) => {
                LCow::Owned(reader.clone().read(()))
            }
            LCow::Owned(MreaSection::Lights(lights)) => LCow::Owned(lights),
            LCow::Borrowed(MreaSection::Lights(lights)) => LCow::Borrowed(lights),
            _ => panic!(),
        }
    }

    pub fn lights_section_mut(&mut self) -> &mut Lights<'r> {
        self.sections.as_mut_vec()[self.lights_section_idx as usize].convert_to_lights()
    }
}

#[derive(Debug, Clone)]
pub enum MreaSection<'r> {
    Unknown(Reader<'r>),
    Scly(Scly<'r>),
    Lights(Lights<'r>),
}

impl<'r> MreaSection<'r> {
    pub fn convert_to_scly(&mut self) -> &mut Scly<'r> {
        *self = match *self {
            MreaSection::Unknown(ref reader) => MreaSection::Scly(reader.clone().read(())),
            MreaSection::Scly(ref mut scly) => return scly,
            _ => panic!(),
        };
        match *self {
            MreaSection::Scly(ref mut scly) => scly,
            _ => panic!(),
        }
    }

    pub fn convert_to_lights(&mut self) -> &mut Lights<'r> {
        *self = match *self {
            MreaSection::Unknown(ref reader) => MreaSection::Lights(reader.clone().read(())),
            MreaSection::Lights(ref mut lights) => return lights,
            _ => panic!(),
        };
        match *self {
            MreaSection::Lights(ref mut lights) => lights,
            _ => panic!(),
        }
    }
}

impl<'r> Readable<'r> for MreaSection<'r> {
    type Args = u32;
    fn read_from(reader: &mut Reader<'r>, size: u32) -> Self {
        let res = MreaSection::Unknown(reader.truncated(size as usize));
        reader.advance(size as usize);
        res
    }

    fn size(&self) -> usize {
        match *self {
            MreaSection::Unknown(ref reader) => reader.len(),
            MreaSection::Scly(ref scly) => scly.size(),
            MreaSection::Lights(ref lights) => lights.size(),
        }
    }
}

impl Writable for MreaSection<'_> {
    fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64> {
        match *self {
            MreaSection::Unknown(ref reader) => {
                writer.write_all(reader)?;
                Ok(reader.len() as u64)
            }
            MreaSection::Scly(ref scly) => scly.write_to(writer),
            MreaSection::Lights(ref lights) => lights.write_to(writer),
        }
    }
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Lights<'r> {
    #[auto_struct(expect = 0xBABEDEAD)]
    magic: u32,

    #[auto_struct(derive = light_layers.len() as u32)]
    pub lights_count: u32,
    #[auto_struct(init = (lights_count as usize, ()))]
    pub light_layers: LazyArray<'r, LightLayer>,

    #[auto_struct(pad_align = 32)]
    _pad: (),
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct LightLayer {
    pub light_type: u32,

    pub color: GenericArray<f32, U3>,
    pub position: GenericArray<f32, U3>,
    pub direction: GenericArray<f32, U3>,

    pub brightness: f32,
    pub spot_cutoff: f32,
    pub unknown0: f32,
    pub unknown1: u8,
    pub unknown2: f32,
    pub falloff_type: u32,
    pub unknown3: f32,
}
