use std::cmp::Ordering;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use std::sync::Arc;

#[derive(Clone, Debug, Hash, Eq)]
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



impl PartialEq<ArcStr> for ArcStr {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || (self.0.as_ref() == other.0.as_ref())
    }
}

impl PartialOrd<ArcStr> for ArcStr {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0.as_ref().cmp(other.0.as_ref()))
    }
}
impl Ord for ArcStr {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.as_ref().cmp(other.0.as_ref())
    }
}

impl AsRef<str> for ArcStr {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}
