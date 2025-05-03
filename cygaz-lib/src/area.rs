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