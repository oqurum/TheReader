use std::rc::Rc;

use common_local::{reader::{ReaderColor, ReaderLoadType}, MemberReaderPreferences};

use super::LayoutDisplay;



#[derive(Clone)]
/// Immutable settings for the reader.
pub struct SharedReaderSettings(pub Rc<ReaderSettings>);

impl SharedReaderSettings {
    pub fn new(value: ReaderSettings) -> Self {
        Self(Rc::new(value))
    }
}

impl std::ops::Deref for SharedReaderSettings {
    type Target = ReaderSettings;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq for SharedReaderSettings {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}


#[derive(Default, PartialEq)]
pub struct ReaderSettings {
    pub type_of: ReaderLoadType,
    pub color: ReaderColor,

    pub animate_page_transitions: bool,
    pub default_full_screen: bool,
    pub auto_full_screen: bool,
    pub display: LayoutDisplay,
    pub show_progress: bool,

    pub dimensions: (i32, i32),
}

impl From<MemberReaderPreferences> for ReaderSettings {
    fn from(value: MemberReaderPreferences) -> Self {
        Self {
            type_of: value.load_type,
            color: value.color,
            animate_page_transitions: value.animate_page_transitions,
            default_full_screen: value.default_full_screen,
            auto_full_screen: value.auto_full_screen,
            display: LayoutDisplay::from(value.display_type),
            show_progress: value.always_show_progress,
            dimensions: (value.width as i32, value.height as i32),
        }
    }
}

