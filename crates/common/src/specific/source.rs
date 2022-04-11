use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Source {
	pub agent: String,
	pub value: String,
}

impl ToString for Source {
    fn to_string(&self) -> String {
        format!("{}:{}", self.agent, self.value)
    }
}

impl TryFrom<&str> for Source {
	type Error = anyhow::Error;

	fn try_from(value: &str) -> Result<Self, Self::Error> {
		let (source, value) = value.split_once(':')
			.ok_or_else(|| anyhow::anyhow!("Missing ':' from Source"))?;

		Ok(Self {
			agent: source.to_owned(),
			value: value.to_owned(),
		})
	}
}

impl TryFrom<String> for Source {
	type Error = anyhow::Error;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		let (source, value) = value.split_once(':')
			.ok_or_else(|| anyhow::anyhow!("Missing ':' from Source"))?;

		Ok(Self {
			agent: source.to_owned(),
			value: value.to_owned(),
		})
	}
}

impl<'de> Deserialize<'de> for Source {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
		let resp = String::deserialize(deserializer)?;
		Ok(Self::try_from(resp).unwrap())
	}
}

impl Serialize for Source {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
		serializer.serialize_str(&self.to_string())
	}
}