use reader_writer::FourCC;
pub use resource_info_table_macro::resource_info;

#[derive(Copy, Clone, Debug)]
pub struct ResourceInfo {
    pub long_name: &'static str,
    pub short_name: Option<&'static str>,
    pub res_id: u32,
    pub fourcc: FourCC,
    pub paks: &'static [&'static [u8]],
}

impl From<ResourceInfo> for (&'_ [&'_ [u8]], u32, FourCC) {
    fn from(val: ResourceInfo) -> Self {
        (val.paks, val.res_id, val.fourcc)
    }
}

impl From<ResourceInfo> for (u32, FourCC) {
    fn from(val: ResourceInfo) -> Self {
        (val.res_id, val.fourcc)
    }
}

impl From<ResourceInfo> for (&'_ [u8], u32) {
    fn from(val: ResourceInfo) -> Self {
        assert_eq!(val.paks.len(), 1);
        (val.paks[0], val.res_id)
    }
}
