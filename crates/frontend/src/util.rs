use std::ops::{Deref, DerefMut};



/// Truncate string based off of char indices instead of bytes.
pub fn truncate_on_indices(s: &mut String, max_chars: usize) {
	if let Some((new_len, _)) = s.char_indices().nth(max_chars) {
		s.truncate(new_len);
	}
}




// TODO: Implement for Frontend Editing.
pub struct EditManager<T: Clone + PartialEq> {
	changed: T,
	original: T,
}

impl<T: Clone + PartialEq> EditManager<T> {
	pub fn new(value: T) -> Self {
		Self {
			original: value.clone(),
			changed: value,
		}
	}

	pub fn is_edited(&self) -> bool {
		self.changed != self.original
	}

	pub fn into_changed(self) -> T {
		self.changed
	}

	pub fn into_value(self) -> T {
		self.original
	}
}

impl<T: Clone + PartialEq> Deref for EditManager<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.changed
	}
}

impl<T: Clone + PartialEq> DerefMut for EditManager<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.changed
	}
}