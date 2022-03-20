use std::sync::{LockResult, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};
use actix_web::{get, App, HttpResponse, HttpServer, Responder, HttpRequest, web};
use actix_web::body::BoxBody;
use cygaz_lib::{fetch_prices, PETROLEUM_TYPE, PetroleumStation};
use serde::{Deserialize,Serialize};
use log::{info, debug, warn};

#[derive(Clone, Serialize)]
struct PriceList {
    updated_at: u128,
    petroleum_type: u32,
    stations: Box<Vec<PetroleumStation>>,
}

#[derive(Deserialize, Clone, Debug)]
struct Config {
    #[serde(default="default_port")]
    port: u16,
    #[serde(default="default_host")]
    host: String,
    #[serde(default="default_timeout")]
    timeout: u32,
}

struct AppStateWithPrices {
    config: Config,
    unlead95: Mutex<PriceList>,
    unlead98: Mutex<PriceList>,
    diesel_heat: Mutex<PriceList>,
    diesel_auto: Mutex<PriceList>,
    kerosene: Mutex<PriceList>,
}

impl Responder for PriceList {
    type Body = BoxBody;
    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        let body = serde_json::to_string(&self).unwrap();
        HttpResponse::Ok()
            .content_type("application/json")
            .body(body)
    }
}

async fn fetch_petroleum(petroleum_type: u32, timeout: u32, lock: LockResult<MutexGuard<'_, PriceList>>) -> PriceList {
    let epoch = SystemTime::now().duration_since(UNIX_EPOCH);
    let epoch_updated_at = epoch.unwrap().as_millis();
    let mut petroleum_list = lock.unwrap();
    let petr_stations = petroleum_list.stations.clone();
    let petr_updated_at = petroleum_list.updated_at;

    if epoch_updated_at - petr_updated_at > timeout as u128 { // over 10 minutes
        debug!("fetching {} {}-{}={}",
            petroleum_type, epoch_updated_at, petr_updated_at, epoch_updated_at - petr_updated_at);
        let stations = fetch_prices(petroleum_type).await;
        if !stations.is_err() {
            let stations_unwrapped = stations.unwrap();
            debug!("found {} station/prices for {}", stations_unwrapped.len(), petroleum_type);
            *petroleum_list = PriceList { petroleum_type, updated_at: epoch_updated_at, stations: Box::new(stations_unwrapped.clone()) };
            return PriceList { petroleum_type, updated_at: epoch_updated_at, stations: Box::new(stations_unwrapped) };
        } else {
            warn!("error while fetching prices: {}", stations.unwrap_err());
        }
    }

    return PriceList{ petroleum_type, updated_at: petr_updated_at, stations: petr_stations };
}

#[get("/prices/1")]
async fn unlead95(data: web::Data<AppStateWithPrices>) -> impl Responder {
    fetch_petroleum(PETROLEUM_TYPE["UNLEAD_95"], data.config.timeout, data.unlead95.lock()).await
}

#[get("/prices/2")]
async fn unlead98(data: web::Data<AppStateWithPrices>) -> impl Responder {
    fetch_petroleum(PETROLEUM_TYPE["UNLEAD_98"], data.config.timeout, data.unlead98.lock()).await
}

#[get("/prices/3")]
async fn diesel_heat(data: web::Data<AppStateWithPrices>) -> impl Responder {
    fetch_petroleum(PETROLEUM_TYPE["DIESEL_HEAT"], data.config.timeout, data.diesel_heat.lock()).await
}

#[get("/prices/4")]
async fn diesel_auto(data: web::Data<AppStateWithPrices>) -> impl Responder {
    fetch_petroleum(PETROLEUM_TYPE["DIESEL_AUTO"], data.config.timeout, data.diesel_auto.lock()).await
}

#[get("/prices/5")]
async fn kerosene(data: web::Data<AppStateWithPrices>) -> impl Responder {
    fetch_petroleum(PETROLEUM_TYPE["KEROSENE"], data.config.timeout, data.kerosene.lock()).await
}

#[get("/version")]
async fn version() -> impl Responder {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    VERSION
}

fn default_port() -> u16 {
    8080
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_timeout() -> u32 {
    600000
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let config = envy::from_env::<Config>().unwrap();

    let epoch = SystemTime::now().duration_since(UNIX_EPOCH);
    info!("warming up cache [{:?}]", config);

    debug!("warming up unlead 95");
    let unlead95_stations = fetch_prices(PETROLEUM_TYPE["UNLEAD_95"]).await;
    if unlead95_stations.is_err() {
        panic!("error while warming up: {}", unlead95_stations.unwrap_err());
    }

    debug!("unlead 95 cached");

    debug!("warming up unlead 98");
    let unlead98_stations = fetch_prices(PETROLEUM_TYPE["UNLEAD_98"]).await;
    if unlead98_stations.is_err() {
        panic!("error while warming up: {}", unlead98_stations.unwrap_err());
    }
    debug!("unlead 98 cached");

    debug!("warming up diesel heat");
    let diesel_heat_stations = fetch_prices(PETROLEUM_TYPE["DIESEL_HEAT"]).await;
    if diesel_heat_stations.is_err() {
        panic!("error while warming up: {}", diesel_heat_stations.unwrap_err());
    }
    debug!("diesel heat cached");

    debug!("warming up diesel auto");
    let diesel_auto_stations = fetch_prices(PETROLEUM_TYPE["DIESEL_AUTO"]).await;
    if diesel_auto_stations.is_err() {
        panic!("error while warming up: {}", diesel_auto_stations.unwrap_err());
    }
    debug!("diesel auto cached");

    debug!("warming up kerosene");
    let kerosene_stations = fetch_prices(PETROLEUM_TYPE["KEROSENE"]).await;
    if kerosene_stations.is_err() {
        panic!("error while warming up: {}", kerosene_stations.unwrap_err());
    }
    debug!("kerosene cached");

    let updated_at = epoch.unwrap().as_millis();

    let data = web::Data::new(AppStateWithPrices{
        config: config.clone(),
        unlead95: Mutex::new(PriceList {
            petroleum_type: PETROLEUM_TYPE["UNLEAD_95"],
            updated_at,
            stations: Box::new(unlead95_stations.unwrap()),
        }),
        unlead98: Mutex::new(PriceList {
            petroleum_type: PETROLEUM_TYPE["UNLEAD_98"],
            updated_at,
            stations: Box::new(unlead98_stations.unwrap()),
        }),
        diesel_heat: Mutex::new(PriceList {
            petroleum_type: PETROLEUM_TYPE["DIESEL_HEAT"],
            updated_at,
            stations: Box::new(diesel_heat_stations.unwrap()),
        }),
        diesel_auto: Mutex::new(PriceList {
            petroleum_type: PETROLEUM_TYPE["DIESEL_AUTO"],
            updated_at,
            stations: Box::new(diesel_auto_stations.unwrap()),
        }),
        kerosene: Mutex::new(PriceList {
            petroleum_type: PETROLEUM_TYPE["KEROSENE"],
            updated_at,
            stations: Box::new(kerosene_stations.unwrap()),
        }),
    });

    info!("running at port {}", config.port);

    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .service(unlead95)
            .service(unlead98)
            .service(diesel_heat)
            .service(diesel_auto)
            .service(kerosene)
            .service(version)
    })
        .bind((config.host, config.port))?
        .run()
        .await
}