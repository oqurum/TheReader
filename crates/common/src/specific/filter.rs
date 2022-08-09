use common::PersonId;
use serde::{Serialize, Deserialize};

use crate::Result;


#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FilterContainer {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<FilterOperator>,
}

impl FilterContainer {
    pub fn into_urlencoded_vec(self) -> Result<Vec<String>> {
        self.filters.into_iter()
            .map(|v| v.encode())
            .collect()
    }

    pub fn from_vec(value: &[String]) -> Result<Self> {
        Ok(Self {
            filters: value.iter()
                .map(|v| FilterOperator::decode(v))
                .collect::<Result<Vec<_>>>()?
        })
    }


    // Basic Filters

    pub fn add_person_filter(&mut self, id: PersonId) {
        self.filters.push(FilterOperator::new(FilterTableType::Person, FilterModifier::Equal, FilterValue::Value(id.to_string())))
    }

    pub fn add_query_filter(&mut self, value: String) {
        self.filters.push(FilterOperator::new(FilterTableType::Query, FilterModifier::Equal, FilterValue::Value(value)))
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterOperator {
    pub type_of: FilterTableType,

    pub modifier: FilterModifier,

    pub value: FilterValue,
}

impl FilterOperator {
    pub fn new(type_of: FilterTableType, modifier: FilterModifier, value: FilterValue) -> Self {
        Self {
            type_of,
            value,
            modifier,
        }
    }

    pub fn encode(&self) -> Result<String> {
        Ok(urlencoding::encode(&serde_json::to_string(self)?).into_owned())
    }

    pub fn decode(value: &str) -> Result<Self> {
        Ok(serde_json::from_slice(&urlencoding::decode_binary(value.as_bytes()))?)
    }
}



#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FilterModifier {
    IsNull,
    IsNotNull,

    GreaterThan,
    GreaterThanOrEqual,

    LessThan,
    LessThanOrEqual,

    Equal,
    DoesNotEqual,
}


#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FilterTableType {
    Id,
    Source,

    //
    Query,
    Person,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterValue {
    Ignored,

    Value(String),

    List(Vec<ListValue>),
}

impl FilterValue {
    pub fn values(&self) -> Vec<String> {
        match self {
            Self::Ignored => Vec::new(),
            Self::Value(v) => vec![v.clone()],
            Self::List(v) => v.iter().map(|v| v.value.clone()).collect(),
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListValue {
    pub value: String,
    pub label: String,
}