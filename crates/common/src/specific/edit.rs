use std::ops::{Deref, DerefMut};



/// Allows for editing while keeping the original value.
pub struct EditManager<D: Clone> {
    value_original: D,
    value_changed: D,
}

impl<D: Clone> EditManager<D> {
    pub fn new(value: D) -> Self {
        Self {
            value_original: value.clone(),
            value_changed: value,
        }
    }

    pub fn into_original_value(self) -> D {
        self.value_original
    }

    pub fn as_original_value(&self) -> &D {
        &self.value_original
    }

    pub fn into_changed_value(self) -> D {
        self.value_changed
    }

    pub fn as_changed_value(&self) -> &D {
        &self.value_changed
    }
}

impl<D: Clone> Deref for EditManager<D> {
    type Target = D;

    fn deref(&self) -> &Self::Target {
        &self.value_changed
    }
}

impl<D: Clone> DerefMut for EditManager<D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value_changed
    }
}


impl<D: Clone + PartialEq> EditManager<D> {
    pub fn has_changed(&self) -> bool {
        self.value_changed != self.value_original
    }
}

impl<D: Clone + Default> Default for EditManager<D> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}




/// Allows for editing
///
/// When immutably reading from it, it'll read from the previously saved value.
///
/// When mutably reading/writing from it, it'll read/write from/to the changed value.
pub struct EditManagerState<D: Clone> {
    value_original: D,
    value_changed: D,

    is_saved: bool,
    is_changed: bool,
}

impl<D: Clone> EditManagerState<D> {
    pub fn new(value: D) -> Self {
        Self {
            value_original: value.clone(),
            value_changed: value,

            is_saved: false,
            is_changed: false,
        }
    }

    pub fn into_last_saved_value(self) -> D {
        self.value_original
    }

    pub fn into_changed_value(self) -> D {
        self.value_changed
    }

    pub fn has_changed(&self) -> bool {
        self.is_changed
    }

    pub fn save(&mut self) {
        self.is_saved = true;
        self.is_changed = false;
    }
}

impl<D: Clone> Deref for EditManagerState<D> {
    type Target = D;

    fn deref(&self) -> &Self::Target {
        // If we saved it, we'll view from changed since
        // We don't update original until we start writing to it after save.
        if self.is_saved {
            &self.value_changed
        } else {
            &self.value_original
        }
    }
}

impl<D: Clone> DerefMut for EditManagerState<D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Update original and mark as unsaved.
        if self.is_saved {
            self.is_saved = false;
            self.value_original = self.value_changed.clone();
        }

        self.is_changed = true;

        &mut self.value_changed
    }
}

impl<D: Clone + Default> Default for EditManagerState<D> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}