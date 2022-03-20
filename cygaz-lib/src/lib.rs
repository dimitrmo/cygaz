extern crate core;

use phf::phf_map;
use url::{Url};
use scraper::{ElementRef, Html, Selector};
use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};
use std::future::Future;

pub static PETROLEUM_TYPE: phf::Map<&'static str, u32> = phf_map! {
    "UNLEAD_95" => 1,
    "UNLEAD_98" => 2,
    "DIESEL_HEAT" => 3,
    "DIESEL_AUTO" => 4,
    "KEROSENE" => 5,
};

static USER_AGENT_VALUE: &'static str = "Mozilla/4.0 (compatible; MSIE 8.0; Windows NT 6.1; Trident/4.0)";

static PETROLEUM_PRICES_ENDPOINT: &'static str = "https://eforms.eservices.cyprus.gov.cy/MCIT/MCIT/PetroleumPrices";

static TOKEN_SELECTOR: &'static str = "input[name=\"__RequestVerificationToken\"]";

static PRICES_SELECTOR: &'static str = "#petroleumPriceDetailsFootable";

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PetroleumStation {
    brand: String,
    offline: bool,
    company: String,
    address: String,
    latitude: String,
    longitude: String,
    area: String,
    price: f32,
}

fn extract_address(endpoint: &Url, fragment: &ElementRef) -> (String, String, String) {
    let a_selector = Selector::parse("a").unwrap();
    let a_tag = fragment.select(&a_selector).next().unwrap();
    let address = a_tag.inner_html();
    let href = a_tag.value().attr("href").unwrap();
    let url = Url::parse(endpoint.join(href).unwrap().as_str()).unwrap();
    let qs = url.query_pairs().collect::<Vec<_>>();
    let (_key, val) = qs.into_iter().find(|(key, _v)| key == "coordinates").unwrap();
    let mut coordinates = val.split(",").collect::<Vec<_>>();
    if coordinates.len() == 1 {
        coordinates = val.split(" ").collect::<Vec<_>>();
    }

    (address, coordinates[0].to_string(), coordinates[1].to_string())
}

pub fn fetch_prices(petroleum_type: u32) -> impl Future<Output = Result<Vec<PetroleumStation>, String>> {
    async move {
        let client = reqwest::Client::builder()
            .cookie_store(true)
            .build()
            .unwrap();

        let response = client.get(PETROLEUM_PRICES_ENDPOINT)
            .header(USER_AGENT, USER_AGENT_VALUE)
            .send()
            .await;
        if response.is_err() {
            return Err(response.unwrap_err().to_string());
        }

        let body = response.unwrap().text().await;
        if body.is_err() {
            return Err(body.unwrap_err().to_string());
        }

        let document = Html::parse_fragment(body.unwrap().as_str());
        let token_selector = Selector::parse(TOKEN_SELECTOR).unwrap();
        let el = document.select(&token_selector).next().unwrap();
        let token = el.value().attr("value").unwrap();

        let form_data = [
            ("__RequestVerificationToken", &token.to_string()),
            ("Entity.StationCityEnum", &"All".to_string()),
            ("Entity.PetroleumType", &petroleum_type.to_string()),
            ("Entity.StationDistrict", &"".to_string()),
        ];

        let endpoint = Url::parse(PETROLEUM_PRICES_ENDPOINT).unwrap();

        let prices_response = client.post(PETROLEUM_PRICES_ENDPOINT)
            .header(USER_AGENT, USER_AGENT_VALUE)
            .form(&form_data)
            .send()
            .await;
        if prices_response.is_err() {
            return Err(prices_response.unwrap_err().to_string());
        }

        let prices_body = prices_response.unwrap().text().await;
        if prices_body.is_err() {
            return Err(prices_body.unwrap_err().to_string());
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
                    let (address_txt, address_lat, address_lon) =
                        extract_address(&endpoint, &address);
                    // println!("address {}-{}-{}", address_txt, address_lat, address_lon);

                    let area = tds.next().unwrap();
                    // println!("area {}", area.inner_html().trim());

                    let price = tds.next().unwrap();
                    // println!("price {}", price.inner_html().trim().parse::<f32>().unwrap());

                    stations.push(PetroleumStation {
                        brand: brand.inner_html().trim().to_string(),
                        offline: offline.is_some(),
                        company: company.inner_html().trim().to_string(),
                        address: address_txt,
                        latitude: address_lat,
                        longitude: address_lon,
                        area: area.inner_html().trim().to_string(),
                        price: price.inner_html().trim().parse::<f32>().unwrap()
                    });
                }
            }
        }

        Ok(stations)
    }
}

#[cfg(test)]
mod tests {
    use crate::{PETROLEUM_TYPE, fetch_prices};
    #[tokio::test]
    async fn unlead_95_prices_for_cyprus() {
        let stations = fetch_prices(PETROLEUM_TYPE["UNLEAD_95"]).await;
        assert!(stations.len() > 0);
    }
    #[tokio::test]
    async fn unlead_98_prices_for_cyprus() {
        let stations = fetch_prices(PETROLEUM_TYPE["UNLEAD_98"]).await;
        assert!(stations.len() > 0);
    }
    #[tokio::test]
    async fn diesel_heat_prices_for_cyprus() {
        let stations = fetch_prices(PETROLEUM_TYPE["DIESEL_HEAT"]).await;
        assert!(stations.len() > 0);
    }
    #[tokio::test]
    async fn diesel_auto_prices_for_cyprus() {
        let stations = fetch_prices(PETROLEUM_TYPE["DIESEL_AUTO"]).await;
        assert!(stations.len() > 0);
    }
    #[tokio::test]
    async fn kerosene_prices_for_cyprus() {
        let stations = fetch_prices(PETROLEUM_TYPE["KEROSENE"]).await;
        assert!(stations.len() > 0);
    }
}
