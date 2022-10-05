use num_enum::{IntoPrimitive, FromPrimitive};



#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, IntoPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum SectionDisplay {
    Single,
    #[default]
    Double,
    Scroll,
}