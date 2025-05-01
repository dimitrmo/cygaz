use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use cygaz_lib::{fetch_areas_for_district, fetch_prices, PetroleumStation, PetroleumType};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock, RwLock};
use std::thread;
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};
use chrono::{DateTime};
use tokio_cron_scheduler::{Job, JobScheduler};
use cygaz_lib::district::{District, DISTRICTS};

static READY: OnceLock<bool> = OnceLock::new();

fn default_port() -> u16 {
    8080
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

#[derive(Deserialize, Clone, Debug)]
struct Config {
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default = "default_host")]
    host: String,
}

#[derive(Debug, Clone, Serialize)]
struct PriceListV2 {
    updated_at: u128,
    updated_at_str: String,
    prices: HashMap<String, HashSet<PetroleumStation>>,
}

impl PriceListV2 {
    pub fn now() -> (u128, String) {
        let epoch = SystemTime::now().duration_since(UNIX_EPOCH);
        let epoch_updated_at = epoch.unwrap().as_millis();
        let datetime = millis_to_datetime(epoch_updated_at);
        (epoch_updated_at, datetime)
    }
}

impl Default for PriceListV2 {
    fn default() -> Self {
        let t = PriceListV2::now();
        Self {
            updated_at: t.0,
            updated_at_str: t.1,
            prices: Default::default()
        }
    }
}

struct AppState {
    /*
    unlead95: PriceList,
    unlead98: PriceList,
    diesel_heat: PriceList,
    diesel_auto: PriceList,
    kerosene: PriceList,
    */
    //
    areas: Arc<RwLock<HashMap<String, District>>>,
    prices: Arc<RwLock<PriceListV2>>
}

fn millis_to_datetime(millis: u128) -> String {
    let secs = (millis / 1000) as i64;
    let datetime_utc = DateTime::from_timestamp(secs, 0).unwrap_or_default();
    datetime_utc.format("%Y-%m-%d %H:%M:%S%.3f UTC").to_string()
}

fn refresh_districts(
    state: web::Data<AppState>
) {
    debug!("refreshing districts");

    let t = thread::spawn(|| {
        let mut output: HashMap<String, District> = HashMap::new();

        for district in DISTRICTS.iter() {
            let areas = fetch_areas_for_district(district.name_en.clone()).unwrap_or_default();
            for area in areas {
                output.insert(area.text, district.clone());
                output.insert(area.value, district.clone());
            }
        }

        output
    });

    let result = t.join().unwrap_or_default();

    let mut lock = state.areas.write().unwrap();

    *lock = result
}

fn find_district(
    area: &String,
    districts: &HashMap<String, District>
) -> District {
    let district = districts.get(area);
    if district.is_some() {
        return district.unwrap().clone()
    }

    District::unknown()
}

fn find_areas_for_district(
    district: &District,
    districts: &HashMap<String, District>
) -> Vec<String> {
    return districts.iter().filter_map(|z| {
        if z.1.eq(district) {
            return Some(z.0.clone())
        }

        return None
    }).collect();
}

fn refresh_price_for_petroleum_type(
    state: web::Data<AppState>,
    p_type: PetroleumType
) -> Vec<PetroleumStation> {
    debug!("warming up {}", p_type);

    let p_state = state.clone();
    let p_handler = thread::spawn(move || {
        let mut prices = fetch_prices(p_type).unwrap_or_else(|err| {
            debug!("Error fetching prices for {}: {}", err, p_type);
            vec![]
        });

        let areas = p_state.areas.read().unwrap();
        for price in prices.iter_mut() {
            price.district = Some(find_district(&price.area, &areas));
        }

        prices
    });

    let result = p_handler.join().unwrap_or_default();

    result
}

fn refresh_prices(
    state: web::Data<AppState>
) {
    debug!("refreshing prices");

    let unlead95_stations = refresh_price_for_petroleum_type(
        state.clone(),
        PetroleumType::Unlead95
    );

    debug!("downloaded {} stations for {}", unlead95_stations.len(), PetroleumType::Unlead95);

    let unlead98_stations = refresh_price_for_petroleum_type(
        state.clone(),
        PetroleumType::Unlead98
    );

    /*
    debug!("downloaded {} stations for {}", unlead98_stations.len(), PetroleumType::Unlead98);

    let diesel_heat_stations = refresh_price_for_petroleum_type(
        state.clone(),
        PetroleumType::DieselHeat
    );

    debug!("downloaded {} stations for {}", diesel_heat_stations.len(), PetroleumType::DieselHeat);

    let diesel_auto_stations = refresh_price_for_petroleum_type(
        state.clone(),
        PetroleumType::DieselAuto
    );

    debug!("downloaded {} stations for {}", diesel_auto_stations.len(), PetroleumType::DieselAuto);

    let kerosene_stations = refresh_price_for_petroleum_type(
        state.clone(),
        PetroleumType::Kerosene
    );

    debug!("downloaded {} stations for {}", kerosene_stations.len(), PetroleumType::Kerosene);
    */

    let mut price_list = state.prices.write().unwrap();

    for station in unlead95_stations.iter()
            .chain(unlead98_stations.iter())
            // .chain(diesel_heat_stations.iter())
            // .chain(diesel_auto_stations.iter())
            //.chain(kerosene_stations.iter()) {
    {
        if let Some(district) = &station.district {
            if !price_list.prices.contains_key(&district.id) {
                price_list.prices.insert(district.id.clone(), Default::default());
            }

            if let Some(stations) = price_list.prices.get_mut(&district.id) {
                match stations.contains(station) {
                    true => {
                        let mut existing = stations.take(station).unwrap();
                        let mut prices = existing.prices;

                        for price in &station.prices {
                            if !prices.contains(&price) {
                                prices.push(*price);
                            }
                        }

                        existing.prices = prices;
                        existing.district = None;
                        stations.insert(existing);
                    }
                    false => {
                        stations.insert(station.clone());
                    }
                }
            }
        }
    }
}

#[get("/prices")]
async fn get_prices(data: web::Data<AppState>) -> impl Responder {
    let prices = data.prices.read().unwrap();
    actix_web::web::Json(prices.clone())
}

#[get("/districts")]
async fn get_districts(
    data: web::Data<AppState>,
) -> impl Responder {
    let areas = data.areas.read().unwrap();
    let mut districts = DISTRICTS.clone();

    for district in &mut districts {
        district.areas = Some(find_areas_for_district(&district, &areas));
    }

    actix_web::web::Json(districts)
}

#[get("/districts/{id}")]
async fn get_district_by_id(
    path: web::Path<String>,
    data: web::Data<AppState>,
) -> impl Responder {
    let id = path.into_inner();
    let areas = data.areas.read().unwrap();
    let mut found_district = District::unknown();

    for district in DISTRICTS.iter() {
        if district.id.eq(&id) {
            found_district = district.clone();
            break;
        }
    }

    found_district.areas = Some(find_areas_for_district(&found_district, &areas));
    actix_web::web::Json(found_district)
}

/*
#[get("/prices/1")]
async fn get_unlead95(data: web::Data<Arc<RwLock<AppState>>>) -> impl Responder {
    let state = data.read().unwrap();
    state.unlead95.clone()
}

#[get("/prices/2")]
async fn get_unlead98(data: web::Data<Arc<RwLock<AppState>>>) -> impl Responder {
    let state = data.read().unwrap();
    state.unlead98.clone()
}

#[get("/prices/3")]
async fn get_diesel_heat(data: web::Data<Arc<RwLock<AppState>>>) -> impl Responder {
    let state = data.read().unwrap();
    state.diesel_heat.clone()
}

#[get("/prices/4")]
async fn get_diesel_auto(data: web::Data<Arc<RwLock<AppState>>>) -> impl Responder {
    let state = data.read().unwrap();
    state.diesel_auto.clone()
}

#[get("/prices/5")]
async fn get_kerosene(data: web::Data<Arc<RwLock<AppState>>>) -> impl Responder {
    let state = data.read().unwrap();
    state.kerosene.clone()
}*/

#[get("/version")]
async fn get_version() -> impl Responder {
    env!("CARGO_PKG_VERSION")
}

#[get("/ready")]
async fn get_ready() -> impl Responder {
    let ready = *READY.get().unwrap_or(&false);
    if ready {
        return HttpResponse::Ok().json(serde_json::json!({ "ready": true }));
    }

    HttpResponse::BadRequest().json(serde_json::json!({ "ready": false }))
}

async fn setup_cron(state: web::Data<AppState>) -> JobScheduler {
    debug!("setting up cron");

    let scheduler = JobScheduler::new().await.unwrap();

    if let Err(e) = scheduler.add(
        Job::new("0 1,16,31,46 * * * *", move |_uuid, _l| {
            info!("cron trigger to refresh prices");
            let prices = state.clone();
            refresh_prices(prices);

            info!("job finished successfully");
        })
        .unwrap(),
    ).await {
        warn!("error scheduling {:?}", e);
    }

    scheduler
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let raw = envy::from_env::<Config>().unwrap();
    let config = Arc::new(raw);
    let address = format!("{}:{}", config.host, config.port);

    info!("warming up initial cache");

    let state = web::Data::new(AppState {
        areas: Default::default(),
        prices: Default::default(),
    });

    let data = state.clone();

    tokio::spawn(async move {
        refresh_districts(data.clone());
        refresh_prices(data);
        debug!("warm up completed");
        READY.set(true)
    });

    let scheduler = setup_cron(state.clone());

    match scheduler.await.start().await {
        Ok(_) => info!("scheduler started"),
        Err(e) => warn!("failed to start scheduler {:?}", e)
    }

    info!("starting http server @ {}", address.clone());

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .service(get_prices)
            //.service(get_unlead95)
            //.service(get_unlead98)
            //.service(get_diesel_heat)
            //.service(get_diesel_auto)
            //.service(get_kerosene)
            .service(get_districts)
            .service(get_district_by_id)
            .service(get_version)
            .service(get_ready)
    })
        .bind(address)
        .unwrap()
        .run()
        .await.expect("server failed to start")
}
