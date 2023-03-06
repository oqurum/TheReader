use std::{rc::Rc, sync::{RwLock, RwLockReadGuard, RwLockWriteGuard}};

use common_local::{reader::{ReaderColor, ReaderLoadType}, MemberReaderPreferences};
use wasm_bindgen::UnwrapThrowExt;

use super::LayoutDisplay;



#[derive(Clone)]
pub struct SharedReaderSettings(Rc<RwLock<ReaderSettings>>);

impl SharedReaderSettings {
    pub fn new(value: ReaderSettings) -> Self {
        Self(Rc::new(RwLock::new(value)))
    }

    pub fn read(&self) -> RwLockReadGuard<'_, ReaderSettings> {
        self.0.read().unwrap_throw()
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, ReaderSettings> {
        self.0.write().unwrap_throw()
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

