use std::hash::{Hash, Hasher};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use crate::area::Area;

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename(serialize = "lowercase", deserialize = "PascalCase"))]
pub struct District {
    pub id: String,
    #[serde(rename = "district_en")]
    pub name_en: String,
    #[serde(rename = "district_el")]
    pub name_el: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub areas: Option<Vec<Area>>
}

impl PartialEq for District {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for District {
    //
}

impl Hash for District {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl District {
    pub fn new(name_en: String, name_el: String) -> Self {
        Self {
            id: name_en.to_ascii_lowercase(),
            name_en,
            name_el,
            areas: None
        }
    }

    pub fn is_valid(district_id: String) -> bool {
        let mut unknown = Self::unknown();
        unknown.id = district_id;
        DISTRICTS.contains(&unknown)
    }

    pub fn unknown() -> Self {
        Self {
            id: "unknown".to_string(),
            name_en: "Unknown".to_string(),
            name_el: "Αγνωστο".to_string(),
            areas: None
        }
    }
}

lazy_static! {
    pub static ref DISTRICTS: Vec<District> = {
        let mut all: Vec<District> = vec![];
        all.push(District::new("Famagusta".to_string(), "Αμμόχωστος".to_string()));
        all.push(District::new("Larnaca".to_string(), "Λάρνακα".to_string()));
        all.push(District::new("Limassol".to_string(), "Λεμεσός".to_string()));
        all.push(District::new("Nicosia".to_string(), "Λευκωσία".to_string()));
        all.push(District::new("Paphos".to_string(), "Πάφος".to_string()));
        all
    };
}