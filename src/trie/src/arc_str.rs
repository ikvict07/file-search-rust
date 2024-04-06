use serde::{Serialize, Serializer, Deserialize, Deserializer};
use std::sync::Arc;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ArcStr(pub Arc<str>);

impl Serialize for ArcStr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ArcStr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
    {
        Ok(ArcStr(Arc::from(String::deserialize(deserializer)?)))
    }
}