use serde::{Deserialize, Serialize};

use crate::reader::{LayoutType, ReaderColor, ReaderLoadType};

// TODO: I don't want to store it like this but it's easiest way.

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberPreferences {
    pub version: usize,

    pub text_book: TextBookPreferences,
    pub image_book: ImageBookPreferences,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneralBookPreferences {
    /// Always show progress bar.
    pub always_show_progress: bool,

    pub animate_page_transitions: bool,
    /// Automatically enter full screen mode if the screen is too small.
    pub auto_full_screen: bool,
    /// Default to full screen mode.
    pub default_full_screen: bool,

    /// Default book width.
    pub width: u32,
    /// Default book height.
    pub height: u32,

    /// Default book background color.
    pub bg_color: ReaderColor,

    /// Default book layout type.
    pub display_type: LayoutType,
    /// Default book loading type.
    pub load_type: ReaderLoadType,
}

impl Default for GeneralBookPreferences {
    fn default() -> Self {
        Self {
            always_show_progress: false,
            animate_page_transitions: true,
            auto_full_screen: false,
            default_full_screen: false,
            width: 1040,
            height: 548,
            bg_color: ReaderColor::Black,
            display_type: LayoutType::Double,
            load_type: ReaderLoadType::Select,
        }
    }
}

// Reader

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextBookPreferences {
    pub desktop: TextBookInnerPreferences,
    pub mobile: TextBookInnerPreferences,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextBookInnerPreferences {
    pub general: GeneralBookPreferences,
    pub reader: ReaderTextPreferences,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReaderTextPreferences {
    pub text_size: u32,
}

// Images

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageBookPreferences {
    pub desktop: ImageBookInnerPreferences,
    pub mobile: ImageBookInnerPreferences,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageBookInnerPreferences {
    pub general: GeneralBookPreferences,
    pub image: ReaderImagePreferences,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReaderImagePreferences {
    // TODO: zoom_landscape_image: bool,
    // TODO: zoom_start_position: Auto, Left, Right, Center
}
