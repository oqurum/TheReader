use std::{borrow::Cow, path::{Path, Component, PathBuf}};

use xml::{
	EmitterConfig,
	reader::XmlEvent as ReaderEvent,
	writer::XmlEvent as WriterEvent,
	attribute::OwnedAttribute,
	name::OwnedName
};

use crate::Result;


/// Based on https://github.com/danigm/epub-rs/blob/master/src/xmlutils.rs#L229
pub fn update_attributes_with<F>(
	input: &[u8],
	func: F,
	add_css: &[&str],
) -> Result<Vec<u8>> where F: Fn(&OwnedName, OwnedAttribute) -> OwnedAttribute {
	let reader = xml::EventReader::new(input);

	let mut output = Vec::new();
	let mut writer = EmitterConfig::default()
		.perform_indent(true)
		.create_writer(&mut output);

	for event in reader {
		match event {
			Err(e) => eprint!("{}", e),
			Ok(v) => match v {
				ReaderEvent::StartElement { name, attributes, namespace } => {
					let attr = attributes
						.into_iter()
						.map(|attr| func(&name, attr))
						.collect::<Vec<_>>();

					writer.write(WriterEvent::StartElement {
						attributes: Cow::Owned(attr.iter().map(|v| v.borrow()).collect()),
						name: name.borrow(),
						namespace: Cow::Owned(namespace)
					}).unwrap();
				}

				ReaderEvent::EndElement { name } => {
					if name.local_name.to_lowercase() == "head" && !add_css.is_empty() {
                        // injecting here the extra css
                        let mut allcss = add_css.concat();
                        allcss = String::from("*/") + &allcss + "/*";

                        writer.write(WriterEvent::start_element("style")).unwrap();
                        writer.write("/*").unwrap();
                        writer.write(WriterEvent::cdata(&allcss)).unwrap();
                        writer.write("*/").unwrap();
                        writer.write(WriterEvent::end_element()).unwrap();
                    }

                    writer.write(WriterEvent::end_element()).unwrap();
				}

				v => {
					if let Some(v) = v.as_writer_event() {
						writer.write(v).unwrap();
					}
				}
			}
		}
	}

	Ok(output)
}


/// Updates the path `value` to include the internal zip `path`
///
/// Also prepends the specific URI before everything
///
/// Based on existing https://github.com/danigm/epub-rs/blob/master/src/doc.rs#L784
pub fn update_value_with_relative_internal_path(mut file_path: PathBuf, value: &str, prepend_text: Option<&str>) -> String {
	// If it's an external file, return.
	if value.starts_with("http") {
		return value.to_string();
	}

	// remove file name.
	file_path.pop();

	for c in Path::new(value).components() {
		match c {
			// If it's ".." remove a directory.
			Component::ParentDir => { file_path.pop(); }
			// Otherwise add on it.
			Component::Normal(v) => { file_path.push(v); }

			_ => ()
		}
	}

	let path = if cfg!(windows) {
		file_path.display().to_string().replace("\\", "/")
	} else {
		file_path.display().to_string()
	};

	if let Some(mut text) = prepend_text.map(|v| v.to_owned()) {
		if !text.ends_with('/') {
			text.push('/');
		}

		text.push_str(&path);

		text
	} else {
		path
	}
}