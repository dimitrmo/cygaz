use std::hash::{Hash, Hasher};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub struct Area {
    // disabled: Option<bool>,
    // group: Option<String>,
    // selected: Option<bool>,
    #[serde(default)]
    pub name_en: String,
    #[serde(alias = "Value")]
    pub name_el: String,
}

impl PartialEq for Area {
    fn eq(&self, other: &Self) -> bool {
        self.name_en == other.name_en && self.name_el == other.name_el
    }
}

impl Eq for Area {
    //
}

impl Hash for Area {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name_en.hash(state);
        self.name_el.hash(state);
    }
}