use serde::{Serialize, Deserialize};


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

            _ => unimplemented!()
        }
    }
}