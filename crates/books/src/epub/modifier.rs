use std::{
    borrow::Cow,
    path::{Component, Path, PathBuf},
};

use xml::{
    attribute::OwnedAttribute, name::OwnedName, reader::XmlEvent as ReaderEvent,
    writer::XmlEvent as WriterEvent, EmitterConfig, EventWriter,
};

use crate::{Book, Result};

/// Based on https://github.com/danigm/epub-rs/blob/master/src/xmlutils.rs#L229
pub fn update_attributes_with<B, F, S>(
    input: &[u8],
    book: &mut B,
    mut func_mod_attr: F,
    mut skip_and_insert: S,
    add_css: &[&str],
) -> Result<Vec<u8>>
where
    B: Book,
    F: FnMut(&mut B, &OwnedName, OwnedAttribute) -> OwnedAttribute,
    S: FnMut(&mut B, &OwnedName, &[OwnedAttribute], &mut EventWriter<&mut Vec<u8>>) -> bool,
{
    let reader = xml::ParserConfig::new()
        .add_entity("nbsp", " ")
        .add_entity("copy", "©")
        .add_entity("reg", "®")
        .create_reader(input);

    let mut output = Vec::new();
    let mut writer = EmitterConfig::default()
        .perform_indent(true)
        .create_writer(&mut output);

    let mut skipping_name = None;

    for event in reader {
        match event {
            Err(e) => eprintln!("update_attributes_with: {}", e),
            Ok(v) => match v {
                ReaderEvent::StartElement {
                    name,
                    attributes,
                    namespace,
                } => {
                    if skipping_name.is_some() {
                        continue;
                    }

                    if skip_and_insert(book, &name, &attributes, &mut writer) {
                        skipping_name = Some(name.clone());
                        continue;
                    }

                    let attr = attributes
                        .into_iter()
                        .map(|attr| func_mod_attr(book, &name, attr))
                        .collect::<Vec<_>>();

                    writer
                        .write(WriterEvent::StartElement {
                            attributes: Cow::Owned(attr.iter().map(|v| v.borrow()).collect()),
                            name: name.borrow(),
                            namespace: Cow::Owned(namespace),
                        })
                        .unwrap();
                }

                ReaderEvent::EndElement { name } => {
                    if let Some(name_match) = skipping_name.as_ref() {
                        if &name == name_match {
                            skipping_name = None;
                        }

                        continue;
                    }

                    if name.local_name.to_lowercase() == "head" && !add_css.is_empty() {
                        // injecting here the extra css
                        let allcss = add_css.concat();

                        writer.write(WriterEvent::start_element("style")).unwrap();
                        writer.write(WriterEvent::characters(&allcss)).unwrap();
                        writer.write(WriterEvent::end_element()).unwrap();
                    }

                    writer.write(WriterEvent::end_element()).unwrap();
                }

                v => {
                    if skipping_name.is_some() {
                        continue;
                    }

                    if let Some(v) = v.as_writer_event() {
                        writer.write(v).unwrap();
                    }
                }
            },
        }
    }

    Ok(output)
}

/// Updates the path `value` to include the internal zip `path`
///
/// Also prepends the specific URI before everything
///
/// Based on existing https://github.com/danigm/epub-rs/blob/master/src/doc.rs#L784
pub fn update_value_with_relative_internal_path(
    mut file_path: PathBuf,
    value: &str,
    prepend_text: Option<&str>,
) -> String {
    // If it's an external file, return.
    if value.starts_with("http") {
        return value.to_string();
    }

    // remove file name.
    file_path.pop();

    for c in Path::new(value).components() {
        match c {
            // If it's ".." remove a directory.
            Component::ParentDir => {
                file_path.pop();
            }
            // Otherwise add on it.
            Component::Normal(v) => {
                file_path.push(v);
            }

            _ => (),
        }
    }

    let path = if cfg!(windows) {
        file_path.display().to_string().replace('\\', "/")
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
