use convert_case::{Case, Casing};
use ordered_float::NotNan;
use serde::{Serialize, Serializer};
use serde::ser::SerializeStruct;
use crate::PetroleumType;

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub struct PetroleumPrice {
    pub p_type: PetroleumType,
    pub value: NotNan<f32>,
}

impl Serialize for PetroleumPrice {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        let mut s = serializer.serialize_struct("PetroleumPrice", 3)?;
        let id = format!("{}", self.p_type);
        s.serialize_field("id", &id.to_case(Case::Snake))?;
        s.serialize_field("label", &self.p_type.to_string())?;
        s.serialize_field("value", &self.value.into_inner())?;
        s.end()
    }
}

impl PetroleumPrice {
    pub fn new(p_type: PetroleumType, price: f32) -> Self {
        Self {
            p_type,
            value: NotNan::new(price).unwrap()
        }
    }
}

/*
pub fn serialize_prices<S>(vec: &Vec<PetroleumPrice>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut map = serializer.serialize_map(Some(vec.len()))?;
    for item in vec {
        map.serialize_entry(&item.p_type, &item.value.serialize(&serializer))?;
    }

    map.end()
}*/
