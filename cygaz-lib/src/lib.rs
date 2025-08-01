pub mod district;
pub mod price;
pub mod station;
pub mod area;

use url::Url;
use serde_json::json;
use std::fmt::{Display};
use std::string::ToString;
use reqwest::header::USER_AGENT;
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use any_ascii::any_ascii;
use crate::area::Area;
use crate::price::PetroleumPrice;
use crate::station::PetroleumStation;

#[derive(Eq, PartialEq, Debug, Copy, Clone, Serialize, Deserialize)]
pub enum PetroleumType {
    Unlead95 = 1,
    Unlead98 = 2,
    DieselHeat = 3,
    DieselAuto = 4,
    Kerosene = 5,
}

impl Display for PetroleumType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PetroleumType::Unlead95 => write!(f, "Unlead 95"),
            PetroleumType::Unlead98 => write!(f, "Unlead 98"),
            PetroleumType::DieselHeat => write!(f, "Diesel Heat"),
            PetroleumType::DieselAuto => write!(f, "Diesel Auto"),
            PetroleumType::Kerosene => write!(f, "Unlead Kerosene"),
        }
    }
}

static GET_STATION_DISTRICT_ENDPOINT: &'static str =
    "https://eforms.eservices.cyprus.gov.cy/MCIT/MCIT/PetroleumPrices/GetStationDistrict";

static USER_AGENT_VALUE: &'static str =
    "Mozilla/4.0 (compatible; MSIE 8.0; Windows NT 6.1; Trident/4.0)";

static PETROLEUM_PRICES_ENDPOINT: &'static str =
    "https://eforms.eservices.cyprus.gov.cy/MCIT/MCIT/PetroleumPrices";

static TOKEN_SELECTOR: &'static str = "input[name=\"__RequestVerificationToken\"]";

static PRICES_SELECTOR: &'static str = "#petroleumPriceDetailsFootable";

#[derive(Clone, Debug)]
pub struct CyGazError(String);

impl Display for CyGazError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn extract_address(endpoint: &Url, fragment: &ElementRef) -> Result<(String, String, String), CyGazError> {
    let a_selector = match Selector::parse("a") {
        Ok(selector) => selector,
        Err(err) => {
            return Err(CyGazError(format!("Parse error {}", err)));
        }
    };

    let a_tag = match fragment.select(&a_selector).next() {
        Some(addr) => addr,
        None => {
            return Err(CyGazError(format!("Select error for address {:?}", fragment.clone())));
        }
    };

    let address = a_tag.inner_html();
    let href = a_tag.value().attr("href").unwrap();
    let url = Url::parse(endpoint.join(href).unwrap().as_str()).unwrap();
    let qs = url.query_pairs().collect::<Vec<_>>();
    let (_key, val) = qs
        .into_iter()
        .find(|(key, _v)| key == "coordinates")
        .unwrap();
    let mut coordinates = val.split(",").collect::<Vec<_>>();
    if coordinates.len() == 1 {
        coordinates = val.split(" ").collect::<Vec<_>>();
    }

    Ok((
        address,
        coordinates[0].to_string(),
        coordinates[1].to_string(),
    ))
}

pub fn fetch_areas_for_district(district: String) -> Result<Vec<Area>, CyGazError> {
    let client = reqwest::blocking::Client::builder()
        .cookie_store(true)
        .build()
        .unwrap();

    let payload = json!({
        "city": district
    });

    let response = client
        .post(GET_STATION_DISTRICT_ENDPOINT)
        .json(&payload)
        .header(USER_AGENT, USER_AGENT_VALUE)
        .send();
    if response.is_err() {
        return Err(CyGazError(response.unwrap_err().to_string()));
    }

    let data = response.unwrap().json::<Vec<Area>>();
    if data.is_err() {
        return Err(CyGazError(data.unwrap_err().to_string()));
    }

    let mut areas: Vec<Area> = vec![];
    for area in data.unwrap().iter_mut() {
        let (name_el, name_en) = transliterate(area.name_el.as_str());
        area.name_el = name_el;
        area.name_en = name_en;
        areas.push(area.to_owned());
    }

    Ok(areas)
}

fn transliterate(original: &str) -> (String, String) {
    let english = any_ascii(original);
    (original.to_string(), english)
}

pub fn fetch_prices(petroleum_type: PetroleumType) -> Result<Vec<PetroleumStation>, CyGazError> {
    let client = reqwest::blocking::Client::builder()
        .cookie_store(true)
        .build()
        .unwrap();

    let response = client
        .get(PETROLEUM_PRICES_ENDPOINT)
        .header(USER_AGENT, USER_AGENT_VALUE)
        .send();
    if response.is_err() {
        return Err(CyGazError(response.unwrap_err().to_string()));
    }

    let body = response.unwrap().text();
    if body.is_err() {
        return Err(CyGazError(body.unwrap_err().to_string()));
    }

    let document = Html::parse_fragment(body.unwrap().as_str());
    let token_selector = Selector::parse(TOKEN_SELECTOR).unwrap();
    let el = document.select(&token_selector).next().unwrap();
    let token = el.value().attr("value").unwrap();

    let form_data = [
        ("__RequestVerificationToken", &token.to_string()),
        ("Entity.StationCityEnum", &"All".to_string()),
        (
            "Entity.PetroleumType",
            &format!("{}", petroleum_type as i32),
        ),
        ("Entity.StationDistrict", &"".to_string()),
    ];

    let endpoint = Url::parse(PETROLEUM_PRICES_ENDPOINT).unwrap();

    let prices_response = client
        .post(PETROLEUM_PRICES_ENDPOINT)
        .header(USER_AGENT, USER_AGENT_VALUE)
        .form(&form_data)
        .send();
    if prices_response.is_err() {
        return Err(CyGazError(prices_response.unwrap_err().to_string()));
    }

    let prices_body = prices_response.unwrap().text();
    if prices_body.is_err() {
        return Err(CyGazError(prices_body.unwrap_err().to_string()));
    }

    let mut stations: Vec<PetroleumStation> = Vec::new();

    let prices_document = Html::parse_fragment(prices_body.unwrap().as_str());
    let table_selector = Selector::parse(PRICES_SELECTOR).unwrap();
    let table_tbody_select = Selector::parse("tbody").unwrap();
    let table_tr_select = Selector::parse("tr").unwrap();
    let table_td_select = Selector::parse("td").unwrap();
    for table in prices_document.select(&table_selector) {
        for tbody in table.select(&table_tbody_select) {
            for tr in tbody.select(&table_tr_select) {
                let mut tds = tr.select(&table_td_select);

                let brand = tds.next().unwrap();
                // println!("brand {}", brand.inner_html().trim());

                let offline = brand.value().classes().find(|c| *c == "isOffLine");
                // println!("offline {}", offline.is_some());

                let company = tds.next().unwrap();
                // println!("company {}", company.inner_html().trim());

                let address = tds.next().unwrap();
                let (address_txt, address_lat, address_lon) = match extract_address(&endpoint, &address) {
                    Ok(result) => result,
                    Err(_) => {
                        // println!("error {}", err);
                        continue;
                    }
                };

                let area = tds.next().unwrap();
                // println!("area {}", area.inner_html().trim());

                let price = tds.next().unwrap();
                // println!("price {}", price.inner_html().trim().parse::<f32>().unwrap());

                let p_price = PetroleumPrice::new(
                    petroleum_type,
                    price.inner_html().trim().to_string()
                );

                let (area_el, area_en) = transliterate(area.inner_html().trim());

                let station = PetroleumStation {
                    brand: brand.inner_html().trim().to_string(),
                    offline: offline.is_some(),
                    company: company.inner_html().trim().to_string(),
                    address: address_txt,
                    latitude: address_lat,
                    longitude: address_lon,
                    area_en,
                    area_el,
                    prices: vec![p_price],
                    district: None,
                };

                stations.push(station);
            }
        }
    }

    Ok(stations)
}

#[cfg(test)]
mod tests {
    use crate::{fetch_prices, PetroleumType};

    #[test]
    fn e2e_unlead_95_prices_for_cyprus() {
        let stations = fetch_prices(PetroleumType::Unlead95).unwrap_or_default();
        assert!(stations.len() > 0);
    }

    #[test]
    fn e2e_unlead_98_prices_for_cyprus() {
        let stations = fetch_prices(PetroleumType::Unlead98).unwrap_or_default();
        assert!(stations.len() > 0);
    }
    #[test]
    fn e2e_diesel_heat_prices_for_cyprus() {
        let stations = fetch_prices(PetroleumType::DieselHeat).unwrap_or_default();
        assert!(stations.len() > 0);
    }
    #[test]
    fn e2e_diesel_auto_prices_for_cyprus() {
        let stations = fetch_prices(PetroleumType::DieselAuto).unwrap_or_default();
        assert!(stations.len() > 0);
    }
    #[test]
    fn e2e_kerosene_prices_for_cyprus() {
        let stations = fetch_prices(PetroleumType::Kerosene).unwrap_or_default();
        assert!(stations.len() > 0);
    }
}
