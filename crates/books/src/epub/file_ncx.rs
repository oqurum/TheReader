use std::{
    io::Read,
    sync::{Arc, Mutex},
};

use xml::{reader::XmlEvent, EventReader};

use super::{Parser, XmlElement};
use crate::{Error, Result};

#[derive(Debug, Default)]
pub struct FileNCX {
    pub version: Option<String>,
    pub lang: Option<String>,

    pub head: Vec<NcxHead>,
    pub nav_map: Vec<NavPoint>,
}

impl FileNCX {
    pub fn parse<R: Read>(value: R) -> Result<Self> {
        let mut this = Self::default();

        let mut reader = EventReader::new(value);

        let mut root_package: Option<Arc<Mutex<XmlElement>>> = None;
        let mut appending_to: Vec<Arc<Mutex<XmlElement>>> = Vec::new();

        loop {
            match reader.next()? {
                XmlEvent::StartElement {
                    name,
                    attributes,
                    namespace,
                } => {
                    let this_item = Arc::new(Mutex::new(XmlElement {
                        name,
                        attributes,
                        namespace,
                        value: None,
                        children: Vec::new(),
                    }));

                    if let Some(parent) = appending_to.last() {
                        parent.lock().unwrap().children.push(this_item.clone());
                        appending_to.push(this_item);
                    } else if root_package.is_none() {
                        root_package = Some(this_item.clone());
                        appending_to.push(this_item);
                    }
                }

                XmlEvent::EndElement { .. } => {
                    appending_to.pop();
                }

                XmlEvent::Characters(v) => {
                    if let Some(parent) = appending_to.last() {
                        parent.lock().unwrap().value = Some(v);
                    }
                }

                XmlEvent::EndDocument => break,

                _ => (),
            }
        }

        let package = root_package.ok_or(Error::MissingValueFor("Root Package"))?;
        let package = Arc::try_unwrap(package).unwrap().into_inner().unwrap();

        this.parse(package)?;

        Ok(this)
    }
}

impl Parser for FileNCX {
    fn parse(&mut self, mut element: XmlElement) -> Result<()> {
        self.version = element.take_attribute("version");
        self.lang = element.take_attribute("lang");

        for mut child in element.take_inner_children() {
            if child.name.local_name == "head" {
                let mut head = NcxHead::default();
                head.parse(child)?;
                self.head.push(head);
            } else if child.name.local_name == "navMap" {
                for child in child.take_inner_children() {
                    let mut head = NavPoint::default();
                    head.parse(child)?;
                    self.nav_map.push(head);
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct NcxHead {
    pub items: Vec<NcxHeadMeta>,
}

impl Parser for NcxHead {
    fn parse(&mut self, mut element: XmlElement) -> Result<()> {
        for child in element.take_inner_children() {
            let mut value = NcxHeadMeta::default();
            value.parse(child)?;
            self.items.push(value);
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct NcxHeadMeta {
    pub content: String,
    pub name: String,
}

impl Parser for NcxHeadMeta {
    fn parse(&mut self, mut element: XmlElement) -> Result<()> {
        self.content = element.take_attribute("content").unwrap();
        self.name = element.take_attribute("name").unwrap();

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct NavPoint {
    pub class: String,
    pub id: String,
    pub play_order: String,

    pub nav_label: NcxLabel,
    pub content: NcxContent,
}

impl Parser for NavPoint {
    fn parse(&mut self, mut element: XmlElement) -> Result<()> {
        self.class = element.take_attribute("class").unwrap();
        self.id = element.take_attribute("id").unwrap();
        self.play_order = element.take_attribute("playOrder").unwrap();

        for mut child in element.take_inner_children() {
            if child.name.local_name == "navLabel" {
                self.nav_label.text = child.take_first_children().unwrap().value.unwrap();
            } else if child.name.local_name == "content" {
                self.content.src = child.take_attribute("src").unwrap();
            }
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct NcxLabel {
    pub text: String,
}

impl Parser for NcxLabel {
    fn parse(&mut self, mut element: XmlElement) -> Result<()> {
        self.text = element.take_attribute("text").unwrap();

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct NcxContent {
    pub src: String,
}

impl Parser for NcxContent {
    fn parse(&mut self, mut element: XmlElement) -> Result<()> {
        self.src = element.take_attribute("src").unwrap();

        Ok(())
    }
}
