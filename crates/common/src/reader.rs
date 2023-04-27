use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReaderColor {
    #[default]
    Default,

    Black,
    Dark,

    White,
    Light,

    Custom {
        foreground: String,
        background: String,
    },
}

impl ReaderColor {
    // TODO: Temporary.
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Default,
            1 => Self::Black,

            _ => unimplemented!(),
        }
    }
}

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    TryFromPrimitive,
    IntoPrimitive,
    Serialize_repr,
    Deserialize_repr,
)]
#[repr(u8)]
pub enum ReaderLoadType {
    All = 0,
    #[default]
    Select,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageMovement {
    LeftToRight,
    RightToLeft,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    IntoPrimitive,
    TryFromPrimitive,
    Serialize_repr,
    Deserialize_repr,
)]
#[repr(u8)]
pub enum LayoutType {
    Single = 0,
    Double,
    Scroll,
    Image,
}
