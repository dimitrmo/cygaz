use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};
use chrono::DateTime;
use convert_case::{Case, Casing};
use serde::{Serialize, Serializer};
use serde::ser::SerializeStruct;
use crate::{PetroleumStation, PetroleumType};

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct PetroleumPrice {
    pub p_type: PetroleumType,
    pub value: String,
}

impl Serialize for PetroleumPrice {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        let mut s = serializer.serialize_struct("PetroleumPrice", 3)?;
        let id = format!("{}", self.p_type);
        let kind: usize = self.p_type as usize;
        s.serialize_field("kind", &kind)?;
        s.serialize_field("id", &id.to_case(Case::Snake))?;
        s.serialize_field("label", &self.p_type.to_string())?;
        s.serialize_field("value", &self.value)?;
        s.end()
    }
}

impl PetroleumPrice {
    pub fn new(p_type: PetroleumType, price: String) -> Self {
        Self {
            p_type,
            value: price,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PriceList {
    pub updated_at: u128,
    pub updated_at_str: String,
    pub prices: HashMap<String, HashSet<PetroleumStation>>,
}

fn millis_to_datetime(millis: u128) -> String {
    let secs = (millis / 1000) as i64;
    let datetime_utc = DateTime::from_timestamp(secs, 0).unwrap_or_default();
    datetime_utc.format("%Y-%m-%d %H:%M:%S%.3f UTC").to_string()
}

impl PriceList {
    pub fn now() -> (u128, String) {
        let epoch = SystemTime::now().duration_since(UNIX_EPOCH);
        let epoch_updated_at = epoch.unwrap().as_millis();
        let datetime = millis_to_datetime(epoch_updated_at);
        (epoch_updated_at, datetime)
    }
}

impl Default for PriceList {
    fn default() -> Self {
        let t = PriceList::now();
        Self {
            updated_at: t.0,
            updated_at_str: t.1,
            prices: Default::default()
        }
    }
}
