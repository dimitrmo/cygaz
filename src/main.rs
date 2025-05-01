use cygaz_lib::{fetch_areas_for_district, fetch_prices, station::PetroleumStation, PetroleumType};
use log::{debug, info, warn};
use serde::{Deserialize};
use std::sync::{Arc, OnceLock, RwLock};
use std::thread;
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use axum::{Json, Router};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use serde_json::{json, Value};
use tokio_cron_scheduler::{Job, JobScheduler};
use cygaz_lib::district::{District, DISTRICTS};
use cygaz_lib::price::PriceList;

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

struct AppState {
    areas: Arc<RwLock<HashMap<String, District>>>,
    prices: Arc<RwLock<PriceList>>
}

fn refresh_districts(
    state: Arc<AppState>
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
    state: Arc<AppState>,
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
    state: Arc<AppState>
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

    let mut price_list = state.prices.write().unwrap();

    for station in unlead95_stations.iter()
            .chain(unlead98_stations.iter())
            .chain(diesel_heat_stations.iter())
            .chain(diesel_auto_stations.iter())
            .chain(kerosene_stations.iter()) {
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
                                prices.push(price.clone());
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

    let time = PriceList::now();
    price_list.updated_at = time.0;
    price_list.updated_at_str = time.1;
}

async fn get_prices(
    State(data): State<Arc<AppState>>,
) -> impl IntoResponse {
    let prices = data.prices.read().unwrap();
    (StatusCode::OK, Json(prices.clone()))
}

async fn get_prices_by_district_id(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let default_price = HashSet::<PetroleumStation>::new();

    if !District::is_valid(id.clone()) {
        warn!("district {:?} is invalid", id.clone());
        let time = PriceList::now();
        return (StatusCode::BAD_REQUEST, Json(json!({
            "updated_at": time.0,
            "updated_at_str": time.1,
            "prices": default_price,
        })));
    }

    let lock = state.prices.clone();
    let guard = lock.read().unwrap();
    let prices = guard.prices.get(&id).unwrap_or(&default_price).clone();

    (
        StatusCode::OK,
        Json(json!({
            "updated_at": guard.updated_at,
            "updated_at_str": guard.updated_at_str,
            "prices": prices,
        }))
    )
}

async fn get_districts(
    State(data): State<Arc<AppState>>,
) -> impl IntoResponse {
    let areas = data.areas.read().unwrap();
    let mut districts = DISTRICTS.clone();

    for district in &mut districts {
        district.areas = Some(find_areas_for_district(&district, &areas));
    }

    (StatusCode::OK, Json(districts))
}

async fn get_district_by_id(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let areas = state.areas.read().unwrap();
    let mut found_district = District::unknown();

    for district in DISTRICTS.iter() {
        if district.id.eq(&id) {
            found_district = district.clone();
            break;
        }
    }

    found_district.areas = Some(find_areas_for_district(&found_district, &areas));
    (StatusCode::OK, Json(found_district))
}

async fn get_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

async fn get_ready() -> (StatusCode, Json<Value>) {
    match *READY.get().unwrap_or(&false) {
        true => ( StatusCode::OK, Json(json!({ "ready": true })) ),
        false => ( StatusCode::BAD_REQUEST, Json(json!({ "ready": false })) ),
    }
}

async fn setup_cron(state: Arc<AppState>) -> JobScheduler {
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

    let shared_state = Arc::new(AppState {
        areas: Default::default(),
        prices: Default::default(),
    });

    let data = shared_state.clone();

    tokio::spawn(async move {
        refresh_districts(data.clone());
        refresh_prices(data);
        debug!("warm up completed");
        READY.set(true)
    });

    let scheduler = setup_cron(shared_state.clone());

    match scheduler.await.start().await {
        Ok(_) => info!("scheduler started"),
        Err(e) => warn!("failed to start scheduler {:?}", e)
    }

    info!("starting http server @ {}", address.clone());

    let app = Router::new()
        .route("/version", get(get_version))
        .route("/ready", get(get_ready))
        .route("/prices", get(get_prices))
        .route("/prices/{id}", get(get_prices_by_district_id))
        .route("/districts", get(get_districts))
        .route("/districts/{id}", get(get_district_by_id))
        .with_state(shared_state);

    /*
    HttpServer::new(move || {
        App::new()
            .app_data(shared_state.clone())
            .service(get_prices)
            .service(get_prices_by_district)
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
        .await.expect("server failed to start")*/

    let listener = tokio::net::TcpListener::bind(address).await.unwrap();

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
