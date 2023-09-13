use std::{
    collections::HashMap,
    io::Read,
    sync::{Arc, Mutex},
};

use xml::{
    attribute::OwnedAttribute, name::OwnedName, namespace::Namespace, reader::XmlEvent, EventReader,
};

use crate::{Error, Result};

pub static FILE_EXT: &str = "opf";
pub static MIME_TYPE: &str = "application/oebps-package+xml";

#[derive(Debug, Default)]
pub struct PackageDocument {
    pub attributes: PackageAttributes,

    pub metadata: PackageMetadata,
    pub manifest: PackageManifest,
    pub spine: PackageSpine,
    pub guide: PackageGuide,

    pub collection: Option<Vec<PackageCollection>>,
}

impl PackageDocument {
    pub fn parse<R: Read>(value: R) -> Result<Self> {
        let mut this = Self::default();

        let mut reader = EventReader::new(value);

        let mut root_package: Option<Arc<Mutex<XmlElement>>> = None;
        let mut appending_to: Vec<Arc<Mutex<XmlElement>>> = Vec::new();

        loop {
            match reader.next()? {
                // XmlEvent::StartDocument { version, encoding, standalone } => {
                //     println!("Start: {:?} {:?} {:?}", version, encoding, standalone);
                // }
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
        let mut package = Arc::try_unwrap(package).unwrap().into_inner().unwrap();

        let package_children = package.take_inner_children();

        this.attributes.parse(package)?;

        for element in package_children {
            if let Some(working_on) = WorkingOn::from_str(&element.name.local_name) {
                let parser: &mut dyn Parser = match working_on {
                    WorkingOn::Metadata => &mut this.metadata,
                    WorkingOn::Manifest => &mut this.manifest,
                    WorkingOn::Spine => &mut this.spine,
                    WorkingOn::Guide => &mut this.guide,
                    // WorkingOn::Collection => this.collection,
                    _ => continue,
                };

                parser.parse(element)?;
            }
        }

        // println!("{:#?}", this);

        Ok(this)
    }
}

#[derive(Debug)]
pub struct XmlElement {
    pub name: OwnedName,
    pub attributes: Vec<OwnedAttribute>,
    pub namespace: Namespace,
    pub value: Option<String>,
    pub children: Vec<Arc<Mutex<XmlElement>>>,
}

impl XmlElement {
    pub fn take_attribute(&mut self, value: &str) -> Option<String> {
        let index = self
            .attributes
            .iter()
            .position(|v| v.name.local_name == value)?;

        Some(self.attributes.remove(index).value)
    }

    pub fn take_inner_children(&mut self) -> Vec<XmlElement> {
        self.children
            .drain(..)
            .map(|this| Arc::try_unwrap(this).unwrap().into_inner().unwrap())
            .collect()
    }

    pub fn take_first_children(&mut self) -> Option<XmlElement> {
        if self.children.len() != 0 {
            Some(
                Arc::try_unwrap(self.children.remove(0))
                    .unwrap()
                    .into_inner()
                    .unwrap(),
            )
        } else {
            None
        }
    }
}

#[derive(Clone, Copy)]
enum WorkingOn {
    Attributes,
    Metadata,
    Manifest,
    Spine,
    Guide,
}

impl WorkingOn {
    pub fn from_str(value: &str) -> Option<Self> {
        Some(match value {
            "package" => Self::Attributes,
            "metadata" => Self::Metadata,
            "manifest" => Self::Manifest,
            "spine" => Self::Spine,
            "guide" => Self::Guide,

            _ => return None,
        })
    }
}

#[derive(Debug, Default)]
pub struct PackageAttributes {
    pub namespace: Option<Namespace>,

    pub dir: Option<String>,
    pub id: Option<String>,
    pub prefix: Option<String>,
    pub xml_lang: Option<String>,
    pub unique_identifier: String,
    pub version: String,

    pub other: HashMap<OwnedName, String>,
}

impl Parser for PackageAttributes {
    fn parse(&mut self, element: XmlElement) -> Result<()> {
        self.namespace = Some(element.namespace);

        for attr in element.attributes {
            // println!("{:?}", (attr.name.prefix.as_deref(), attr.name.local_name.as_str()));

            match (attr.name.prefix.as_deref(), attr.name.local_name.as_str()) {
                (None, "dir") => self.dir = Some(attr.value),
                (None, "id") => self.id = Some(attr.value),
                (None, "prefix") => self.prefix = Some(attr.value),
                (Some("xml"), "lang") => self.xml_lang = Some(attr.value),
                (None, "unique-identifier") => self.unique_identifier = attr.value,
                (None, "version") => self.version = attr.value,

                _ => {
                    self.other.insert(attr.name, attr.value);
                }
            }
        }

        // TODO: assertions https://www.w3.org/publishing/epub3/epub-packages.html#sec-package-elem
        Ok(())
    }
}

// CONTENT

#[derive(Debug, Default)]
pub struct PackageMetadata {
    pub meta_items: Vec<MetaItem>,
    pub dcmes_elements: HashMap<String, Vec<DcmesElement>>,
    pub non_prefixed_items: HashMap<String, Vec<XmlElement>>,
    pub opf_items: HashMap<String, Vec<XmlElement>>,
}

impl PackageMetadata {
    // TODO: Make REQUIRED elements functions, along with utilizing meta connections.

    pub fn get_creators(&self) -> Vec<&str> {
        self.dcmes_elements
            .get("creator")
            .map(|v| v.iter().filter_map(|v| v.value.as_deref()).collect())
            .unwrap_or_default()
    }

    pub fn get_ident_pub(&self) -> Option<&str> {
        self.dcmes_elements.get("identifier")?.iter().find_map(|v| {
            if v.id.as_deref() == Some("pub-id") {
                v.value.as_deref()
            } else {
                None
            }
        })
    }

    pub fn get_ident_isbn(&self) -> Option<&str> {
        self.dcmes_elements.get("identifier")?.iter().find_map(|v| {
            if v.id.as_deref() == Some("isbn-id") {
                v.value.as_deref()
            } else {
                None
            }
        })
    }

    // TODO: Actually utilize <meta refines="#dc-name" ..>
}

impl Parser for PackageMetadata {
    fn parse(&mut self, mut element: XmlElement) -> Result<()> {
        for child in element.take_inner_children() {
            match (child.name.prefix.as_deref(), child.name.local_name.as_str()) {
                // FIX: for some files including a sub container filled with dc:names inside it.
                (None, "dc-metadata") => {
                    Self::parse(self, child)?;
                }

                (None, "meta") => self.meta_items.push(MetaItem::try_from(child)?),

                (None, name) => {
                    self.non_prefixed_items
                        .entry(name.to_owned())
                        .or_default()
                        .push(child);
                }

                (Some("dc"), name) => {
                    self.dcmes_elements
                        .entry(name.to_owned())
                        .or_default()
                        .push(DcmesElement::try_from(child)?);
                }

                (Some("opf"), name) => {
                    self.opf_items
                        .entry(name.to_owned())
                        .or_default()
                        .push(child);
                }

                _ => println!(
                    "PackageMetadata::parse(XmlElement): Missing Child Element parse for: {:?}",
                    (child.name.prefix.as_deref(), child.name.local_name.as_str())
                ),
            }
        }

        // TODO: assertions https://www.w3.org/publishing/epub3/epub-packages.html#sec-metadata-elem
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct MetaItem {
    pub dir: Option<String>,
    pub id: Option<String>,
    pub refines: Option<String>,
    pub scheme: Option<String>,
    pub xml_lang: Option<String>,

    pub property: String,

    pub value: Option<String>,

    pub undefined: HashMap<String, String>,
}

impl MetaItem {
    pub fn get(&self, value: &str) -> Option<&str> {
        match value {
            "dir" => self.dir.as_deref(),
            "id" => self.id.as_deref(),
            "refines" => self.refines.as_deref(),
            "scheme" => self.scheme.as_deref(),
            "lang" => self.xml_lang.as_deref(),
            "property" => Some(&self.property),
            "value" => self.value.as_deref(),

            v => self.undefined.get(v).map(|v| v.as_str()),
        }
    }
}

impl TryFrom<XmlElement> for MetaItem {
    type Error = Error;

    fn try_from(elem: XmlElement) -> std::result::Result<Self, Self::Error> {
        let mut this = Self {
            dir: None,
            id: None,
            refines: None,
            scheme: None,
            xml_lang: None,
            property: String::new(),
            value: elem.value,
            undefined: HashMap::new(),
        };

        for attr in elem.attributes {
            match attr.name.local_name.as_str() {
                "dir" => this.dir = Some(attr.value),
                "id" => this.id = Some(attr.value),
                "refines" => this.refines = Some(attr.value),
                "scheme" => this.scheme = Some(attr.value),
                "lang" => this.xml_lang = Some(attr.value),
                "property" => this.property = attr.value,

                _ => {
                    this.undefined.insert(attr.name.local_name, attr.value);
                }
            }
        }

        // TODO: Ensure property is something.
        // TODO: Errors

        Ok(this)
    }
}

#[derive(Debug, Default)]
pub struct PackageManifest {
    pub id: Option<String>,
    pub items: Vec<ManifestItem>,
}

impl PackageManifest {
    pub fn get_item_by_id(&self, value: &str) -> Option<&ManifestItem> {
        self.items.iter().find(|item| item.id == value)
    }

    pub fn get_item_by_href(&self, value: &str) -> Option<&ManifestItem> {
        self.items.iter().find(|item| item.href == value)
    }

    pub fn get_item_by_property(&self, value: &str) -> Option<&ManifestItem> {
        self.items
            .iter()
            .find(|item| item.properties.as_deref() == Some(value))
    }
}

impl Parser for PackageManifest {
    fn parse(&mut self, mut element: XmlElement) -> Result<()> {
        self.id = element
            .attributes
            .iter()
            .find(|v| v.name.local_name == "id")
            .map(|v| v.value.to_owned());

        for child in element.take_inner_children() {
            self.items.push(ManifestItem::try_from(child)?);
        }

        // TODO: assertions | https://www.w3.org/publishing/epub3/epub-packages.html#sec-manifest-elem

        Ok(())
    }
}

#[derive(Debug)]
pub struct ManifestItem {
    pub fallback: Option<String>,
    pub href: String,
    pub id: String,
    pub media_overlay: Option<String>,
    pub media_type: String,
    pub properties: Option<String>,
}

// TODO: Could use serde if I wanted to.
impl TryFrom<XmlElement> for ManifestItem {
    type Error = Error;

    fn try_from(elem: XmlElement) -> std::result::Result<Self, Self::Error> {
        let mut attr = elem
            .attributes
            .into_iter()
            .map(|v| {
                (
                    v.name
                        .prefix
                        .map(|p| format!("{}:{}", p, v.name.local_name.as_str()))
                        .unwrap_or(v.name.local_name),
                    v.value,
                )
            })
            .collect::<HashMap<_, _>>();

        // TODO: Errors

        Ok(Self {
            fallback: attr.remove("fallback"),
            href: attr.remove("href").unwrap(),
            id: attr.remove("id").unwrap(),
            media_overlay: attr.remove("media-overlay"),
            media_type: attr.remove("media-type").unwrap(),
            properties: attr.remove("properties"),
        })
    }
}

#[derive(Debug, Default)]
pub struct PackageSpine {
    pub id: Option<String>,
    pub page_progression_direction: Option<String>,
    pub toc: Option<String>, // LEGACY
    pub items: Vec<SpineItemRef>,
}

impl PackageSpine {
    pub fn position_of_idref(&self, value: &str) -> Option<usize> {
        self.items.iter().position(|item| item.idref == value)
    }
}

impl Parser for PackageSpine {
    fn parse(&mut self, mut element: XmlElement) -> Result<()> {
        for child in element.take_inner_children() {
            self.items.push(SpineItemRef::try_from(child)?);
        }

        // TODO: assertions

        Ok(())
    }
}

#[derive(Debug)]
pub struct SpineItemRef {
    pub id: Option<String>,
    pub idref: String,
    pub linear: Option<String>,
    pub properties: Option<String>,
}

impl SpineItemRef {
    /// https://www.w3.org/publishing/epub3/epub-packages.html#sec-itemref-elem
    pub fn is_linear(&self) -> bool {
        if let Some(linear) = self.linear.as_deref() {
            linear == "yes"
        } else {
            true
        }
    }
}

// TODO: Could use serde if I wanted to.
impl TryFrom<XmlElement> for SpineItemRef {
    type Error = Error;

    fn try_from(elem: XmlElement) -> std::result::Result<Self, Self::Error> {
        let mut attr = elem
            .attributes
            .into_iter()
            .map(|v| {
                (
                    v.name
                        .prefix
                        .map(|p| format!("{}:{}", p, v.name.local_name.as_str()))
                        .unwrap_or(v.name.local_name),
                    v.value,
                )
            })
            .collect::<HashMap<_, _>>();

        // TODO: Errors

        Ok(Self {
            id: attr.remove("id"),
            idref: attr.remove("idref").unwrap(),
            linear: attr.remove("linear"),
            properties: attr.remove("properties"),
        })
    }
}

#[derive(Debug, Default)]
pub struct PackageCollection {
    pub dir: Option<String>,
    pub id: Option<String>,
    pub role: String,
    pub xml_lang: Option<String>,
}

impl Parser for PackageCollection {
    fn parse(&mut self, element: XmlElement) -> Result<()> {
        println!(
            "{:?}",
            (
                element.name.prefix.as_deref(),
                element.name.local_name.as_str()
            )
        );

        // https://www.w3.org/publishing/epub3/epub-packages.html#sec-pkg-collections

        // TODO: assertions

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct PackageGuide {
    pub items: Vec<GuideItemRef>,
}

impl PackageGuide {
    pub fn find_toc(&self) -> Option<&GuideItemRef> {
        self.items.iter().find(|v| v.type_of == "toc")
    }
}

impl Parser for PackageGuide {
    fn parse(&mut self, mut element: XmlElement) -> Result<()> {
        for child in element.take_inner_children() {
            self.items.push(GuideItemRef::try_from(child)?);
        }

        // TODO: assertions

        Ok(())
    }
}

#[derive(Debug)]
pub struct GuideItemRef {
    pub href: String,
    pub type_of: String,
    pub title: String,
}

// TODO: Could use serde if I wanted to.
impl TryFrom<XmlElement> for GuideItemRef {
    type Error = Error;

    fn try_from(elem: XmlElement) -> std::result::Result<Self, Self::Error> {
        let mut attr = elem
            .attributes
            .into_iter()
            .map(|v| {
                (
                    v.name
                        .prefix
                        .map(|p| format!("{}:{}", p, v.name.local_name.as_str()))
                        .unwrap_or(v.name.local_name),
                    v.value,
                )
            })
            .collect::<HashMap<_, _>>();

        // TODO: Errors

        Ok(Self {
            href: attr.remove("href").unwrap(),
            type_of: attr.remove("type").unwrap(),
            title: attr.remove("title").unwrap(),
        })
    }
}

// TODO: Meet specific criteria. | https://www.w3.org/publishing/epub3/epub-packages.html#sec-package-content-conf

#[derive(Debug, Default)]
pub struct PairIdValue {
    pub id: String,
    pub value: String,
}

impl TryFrom<XmlElement> for PairIdValue {
    type Error = Error;

    fn try_from(mut value: XmlElement) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            id: value.attributes.remove(0).value,
            value: value.value.unwrap(),
        })
    } // TODO: Error
}

#[derive(Debug, Clone)]
pub struct DcmesElement {
    pub namespace: Namespace,

    pub dir: Option<String>,
    pub id: Option<String>,
    pub xml_lang: Option<String>,

    pub value: Option<String>,
}

impl TryFrom<XmlElement> for DcmesElement {
    type Error = Error;

    fn try_from(elem: XmlElement) -> std::result::Result<Self, Self::Error> {
        let mut attr = elem
            .attributes
            .into_iter()
            .map(|v| {
                (
                    v.name
                        .prefix
                        .map(|p| format!("{}:{}", p, v.name.local_name.as_str()))
                        .unwrap_or(v.name.local_name),
                    v.value,
                )
            })
            .collect::<HashMap<_, _>>();

        Ok(Self {
            namespace: elem.namespace,
            dir: attr.remove("dir"),
            id: attr.remove("id"),
            xml_lang: attr.remove("xml:lang"),
            value: elem.value,
        })
    } // TODO: Error
}

pub(in crate::epub) trait Parser {
    fn parse(&mut self, element: XmlElement) -> Result<()>;
}
