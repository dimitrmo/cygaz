use std::sync::{LockResult, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::{SystemTime, UNIX_EPOCH};
use actix_web::{get, patch, App, HttpResponse, HttpServer, Responder, HttpRequest, web};
use actix_web::body::BoxBody;
use actix_web::http::StatusCode;
use cygaz_lib::{fetch_prices, PETROLEUM_TYPE, PetroleumStation};
use serde::{Deserialize,Serialize};
use log::{info, debug};

#[derive(Clone, Serialize)]
struct PriceList {
    updated_at: u128,
    petroleum_type: u32,
    stations: Box<Vec<PetroleumStation>>,
}

impl PriceList {
    fn empty(petroleum_type: u32, updated_at: u128) -> Self {
        PriceList{
            petroleum_type,
            updated_at,
            stations: Box::new(vec![]),
        }
    }
}

fn default_port() -> u16 {
    8080
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

#[derive(Deserialize, Clone, Debug)]
struct Config {
    #[serde(default="default_port")]
    port: u16,
    #[serde(default="default_host")]
    host: String,
}

struct AppStateWithPrices {
    unlead95: RwLock<PriceList>,
    unlead98: RwLock<PriceList>,
    diesel_heat: RwLock<PriceList>,
    diesel_auto: RwLock<PriceList>,
    kerosene: RwLock<PriceList>,
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

async fn fetch_petroleum(petroleum_type: u32, lock: LockResult<RwLockReadGuard<'_, PriceList>>) -> PriceList {
    let epoch = SystemTime::now().duration_since(UNIX_EPOCH);
    let epoch_updated_at = epoch.unwrap().as_millis();
    if lock.is_err() {
        return PriceList::empty(petroleum_type, epoch_updated_at);
    }

    let petroleum_list = lock.unwrap();
    let petr_stations = petroleum_list.stations.clone();
    let petr_updated_at = petroleum_list.updated_at;

    return PriceList{ petroleum_type, updated_at: petr_updated_at, stations: petr_stations };
}

async fn refresh_petroleum(petroleum_type: u32, lock: LockResult<RwLockWriteGuard<'_, PriceList>>) {
    let mut petroleum_list = lock.unwrap();
    debug!("fetching {}", petroleum_type);

    let stations = fetch_prices(petroleum_type).await.unwrap_or_default();
    debug!("found {} station/prices for {}", stations.len(), petroleum_type);

    // fetch timestamp
    let epoch = SystemTime::now().duration_since(UNIX_EPOCH);
    let epoch_updated_at = epoch.unwrap().as_millis();

    // update local cache
    *petroleum_list = PriceList { petroleum_type, updated_at: epoch_updated_at,
        stations: Box::new(stations) };
}

#[get("/prices/1")]
async fn unlead95(data: web::Data<AppStateWithPrices>) -> impl Responder {
    fetch_petroleum(PETROLEUM_TYPE["UNLEAD_95"], data.unlead95.read()).await
}

#[patch("/prices/1/refresh")]
async fn refresh_unlead95(data: web::Data<AppStateWithPrices>) -> impl Responder {
    refresh_petroleum(PETROLEUM_TYPE["UNLEAD_95"], data.unlead95.write()).await;
    HttpResponse::Ok().status(StatusCode::NO_CONTENT).finish()
}

#[get("/prices/2")]
async fn unlead98(data: web::Data<AppStateWithPrices>) -> impl Responder {
    fetch_petroleum(PETROLEUM_TYPE["UNLEAD_98"], data.unlead98.read()).await
}

#[patch("/prices/2/refresh")]
async fn refresh_unlead98(data: web::Data<AppStateWithPrices>) -> impl Responder {
    refresh_petroleum(PETROLEUM_TYPE["UNLEAD_98"], data.unlead98.write()).await;
    HttpResponse::Ok().status(StatusCode::NO_CONTENT).finish()
}

#[get("/prices/3")]
async fn diesel_heat(data: web::Data<AppStateWithPrices>) -> impl Responder {
    fetch_petroleum(PETROLEUM_TYPE["DIESEL_HEAT"], data.diesel_heat.read()).await
}

#[patch("/prices/3/refresh")]
async fn refresh_diesel_heat(data: web::Data<AppStateWithPrices>) -> impl Responder {
    refresh_petroleum(PETROLEUM_TYPE["DIESEL_HEAT"], data.diesel_heat.write()).await;
    HttpResponse::Ok().status(StatusCode::NO_CONTENT).finish()
}

#[get("/prices/4")]
async fn diesel_auto(data: web::Data<AppStateWithPrices>) -> impl Responder {
    fetch_petroleum(PETROLEUM_TYPE["DIESEL_AUTO"], data.diesel_auto.read()).await
}

#[patch("/prices/4/refresh")]
async fn refresh_diesel_auto(data: web::Data<AppStateWithPrices>) -> impl Responder {
    refresh_petroleum(PETROLEUM_TYPE["DIESEL_AUTO"], data.diesel_auto.write()).await;
    HttpResponse::Ok().status(StatusCode::NO_CONTENT).finish()
}

#[get("/prices/5")]
async fn kerosene(data: web::Data<AppStateWithPrices>) -> impl Responder {
    fetch_petroleum(PETROLEUM_TYPE["KEROSENE"], data.kerosene.read()).await
}

#[patch("/prices/5/refresh")]
async fn refresh_kerosene(data: web::Data<AppStateWithPrices>) -> impl Responder {
    refresh_petroleum(PETROLEUM_TYPE["KEROSENE"], data.kerosene.write()).await;
    HttpResponse::Ok().status(StatusCode::NO_CONTENT).finish()
}

#[get("/version")]
async fn version() -> impl Responder {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    VERSION
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
        unlead95: RwLock::new(PriceList {
            petroleum_type: PETROLEUM_TYPE["UNLEAD_95"],
            updated_at,
            stations: Box::new(unlead95_stations.unwrap()),
        }),
        unlead98: RwLock::new(PriceList {
            petroleum_type: PETROLEUM_TYPE["UNLEAD_98"],
            updated_at,
            stations: Box::new(unlead98_stations.unwrap()),
        }),
        diesel_heat: RwLock::new(PriceList {
            petroleum_type: PETROLEUM_TYPE["DIESEL_HEAT"],
            updated_at,
            stations: Box::new(diesel_heat_stations.unwrap()),
        }),
        diesel_auto: RwLock::new(PriceList {
            petroleum_type: PETROLEUM_TYPE["DIESEL_AUTO"],
            updated_at,
            stations: Box::new(diesel_auto_stations.unwrap()),
        }),
        kerosene: RwLock::new(PriceList {
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
            .service(refresh_unlead95)
            .service(refresh_unlead98)
            .service(refresh_diesel_heat)
            .service(refresh_diesel_auto)
            .service(refresh_kerosene)
            .service(version)
    })
        .bind((config.host, config.port))?
        .run()
        .await
}
