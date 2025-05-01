use std::hash::{Hash, Hasher};
use serde::Serialize;
use crate::district::District;
use crate::price::PetroleumPrice;

#[derive(Clone, Serialize, Debug)]
pub struct PetroleumStation {
    pub(crate) brand: String,
    pub(crate) offline: bool,
    pub(crate) company: String,
    pub(crate) address: String,
    pub(crate) latitude: String,
    pub(crate) longitude: String,
    pub area: String,
    pub prices: Vec<PetroleumPrice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub district: Option<District>,
}

impl Hash for PetroleumStation {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.latitude.hash(state);
        self.longitude.hash(state);
    }
}

impl PartialEq for PetroleumStation {
    fn eq(&self, other: &Self) -> bool {
        self.latitude == other.latitude && self.longitude == other.longitude
    }
}

impl Eq for PetroleumStation {
    //
}