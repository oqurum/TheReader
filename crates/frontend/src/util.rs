use std::ops::{Deref, DerefMut};

use gloo_utils::window;
use web_sys::Element;


// TODO: Better names

pub fn as_local_path_with_http(value: &str) -> String {
	format!(
		"{}/{}",
		window().location().origin().unwrap(),
		if let Some(v) = value.strip_prefix('/') {
			v
		} else {
			value
		}
	)
}

pub fn as_local_path_without_http(value: &str) -> String {
	format!(
		"{}/{}",
		window().location().hostname().unwrap(),
		if let Some(v) = value.strip_prefix('/') {
			v
		} else {
			value
		}
	)
}


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



pub fn does_parent_contain_class(element: &Element, value: &str) -> bool {
	if element.class_list().contains(value) {
		true
	} else if let Some(element) = element.parent_element() {
		does_parent_contain_class(&element, value)
	} else {
		false
	}
}