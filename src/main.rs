use actix_web::body::BoxBody;
use actix_web::http::StatusCode;
use actix_web::{get, guard, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use cygaz_lib::{fetch_prices, PetroleumStation, PETROLEUM_TYPE};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::sync::{LockResult, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Clone, Serialize)]
struct PriceList {
    updated_at: u128,
    petroleum_type: u32,
    stations: Box<Vec<PetroleumStation>>,
}

impl PriceList {
    fn empty(petroleum_type: u32, updated_at: u128) -> Self {
        PriceList {
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

fn default_uuid() -> String {
    Uuid::new_v4().to_string()
}

#[derive(Deserialize, Clone, Debug)]
struct Config {
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default = "default_host")]
    host: String,
    #[serde(default = "default_uuid")]
    secret: String,
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

fn fetch_petroleum(
    petroleum_type: u32,
    lock: LockResult<RwLockReadGuard<'_, PriceList>>,
) -> PriceList {
    let epoch = SystemTime::now().duration_since(UNIX_EPOCH);
    let epoch_updated_at = epoch.unwrap().as_millis();
    if lock.is_err() {
        return PriceList::empty(petroleum_type, epoch_updated_at);
    }

    let petroleum_list = lock.unwrap();
    let petr_stations = petroleum_list.stations.clone();
    let petr_updated_at = petroleum_list.updated_at;

    PriceList {
        petroleum_type,
        updated_at: petr_updated_at,
        stations: petr_stations,
    }
}

fn refresh_petroleum(petroleum_type: u32, lock: LockResult<RwLockWriteGuard<'_, PriceList>>) {
    let mut petroleum_list = lock.unwrap();
    debug!("refreshing prices for {}", petroleum_type);

    let stations = fetch_prices(petroleum_type).unwrap_or_default();
    debug!(
        "found {} station/prices for {}",
        stations.len(),
        petroleum_type
    );

    // fetch timestamp
    let epoch = SystemTime::now().duration_since(UNIX_EPOCH);
    let epoch_updated_at = epoch.unwrap().as_millis();

    // update local cache
    *petroleum_list = PriceList {
        petroleum_type,
        updated_at: epoch_updated_at,
        stations: Box::new(stations),
    };
}

#[get("/prices/1")]
async fn unlead95(data: web::Data<AppStateWithPrices>) -> impl Responder {
    fetch_petroleum(PETROLEUM_TYPE["UNLEAD_95"], data.unlead95.read())
}

#[get("/prices/2")]
async fn unlead98(data: web::Data<AppStateWithPrices>) -> impl Responder {
    fetch_petroleum(PETROLEUM_TYPE["UNLEAD_98"], data.unlead98.read())
}

#[get("/prices/3")]
async fn diesel_heat(data: web::Data<AppStateWithPrices>) -> impl Responder {
    fetch_petroleum(PETROLEUM_TYPE["DIESEL_HEAT"], data.diesel_heat.read())
}

#[get("/prices/4")]
async fn diesel_auto(data: web::Data<AppStateWithPrices>) -> impl Responder {
    fetch_petroleum(PETROLEUM_TYPE["DIESEL_AUTO"], data.diesel_auto.read())
}

#[get("/prices/5")]
async fn kerosene(data: web::Data<AppStateWithPrices>) -> impl Responder {
    fetch_petroleum(PETROLEUM_TYPE["KEROSENE"], data.kerosene.read())
}

#[get("/version")]
async fn version() -> impl Responder {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    VERSION
}

async fn refresh_unlead95(data: web::Data<AppStateWithPrices>) -> impl Responder {
    thread::spawn(move || {
        refresh_petroleum(PETROLEUM_TYPE["UNLEAD_95"], data.unlead95.write());
    });
    HttpResponse::Ok().status(StatusCode::NO_CONTENT).finish()
}

async fn refresh_unlead98(data: web::Data<AppStateWithPrices>) -> impl Responder {
    thread::spawn(move || {
        refresh_petroleum(PETROLEUM_TYPE["UNLEAD_98"], data.unlead98.write());
    });
    HttpResponse::Ok().status(StatusCode::NO_CONTENT).finish()
}

async fn refresh_diesel_heat(data: web::Data<AppStateWithPrices>) -> impl Responder {
    thread::spawn(move || {
        refresh_petroleum(PETROLEUM_TYPE["DIESEL_HEAT"], data.diesel_heat.write());
    });
    HttpResponse::Ok().status(StatusCode::NO_CONTENT).finish()
}

async fn refresh_diesel_auto(data: web::Data<AppStateWithPrices>) -> impl Responder {
    thread::spawn(move || {
        refresh_petroleum(PETROLEUM_TYPE["DIESEL_AUTO"], data.diesel_auto.write());
    });
    return HttpResponse::Ok().status(StatusCode::NO_CONTENT).finish();
}

async fn refresh_kerosene(data: web::Data<AppStateWithPrices>) -> impl Responder {
    refresh_petroleum(PETROLEUM_TYPE["KEROSENE"], data.diesel_auto.write());
    return HttpResponse::Ok().status(StatusCode::NO_CONTENT).finish();
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let config = envy::from_env::<Config>().unwrap();
    let address = format!("{}:{}", config.host, config.port);
    let epoch = SystemTime::now().duration_since(UNIX_EPOCH);

    info!("warming up cache");

    let unlead95_handler = thread::spawn(|| {
        debug!("warming up unlead 95");
        fetch_prices(PETROLEUM_TYPE["UNLEAD_95"]).unwrap_or_default()
    });

    let unlead98_handler = thread::spawn(|| {
        debug!("warming up unlead 98");
        fetch_prices(PETROLEUM_TYPE["UNLEAD_98"]).unwrap_or_default()
    });

    let diesel_heat_handler = thread::spawn(|| {
        debug!("warming up diesel heat");
        fetch_prices(PETROLEUM_TYPE["DIESEL_HEAT"]).unwrap_or_default()
    });

    let diesel_auto_handler = thread::spawn(|| {
        debug!("warming up diesel auto");
        fetch_prices(PETROLEUM_TYPE["DIESEL_AUTO"]).unwrap_or_default()
    });

    let kerosene_handler = thread::spawn(|| {
        debug!("warming up kerosene");
        fetch_prices(PETROLEUM_TYPE["KEROSENE"]).unwrap_or_default()
    });

    let unlead95_stations = unlead95_handler.join().unwrap();
    let unlead98_stations = unlead98_handler.join().unwrap();
    let diesel_heat_stations = diesel_heat_handler.join().unwrap();
    let diesel_auto_stations = diesel_auto_handler.join().unwrap();
    let kerosene_stations = kerosene_handler.join().unwrap();

    let updated_at = epoch.unwrap().as_millis();

    let data = web::Data::new(AppStateWithPrices {
        unlead95: RwLock::new(PriceList {
            petroleum_type: PETROLEUM_TYPE["UNLEAD_95"],
            updated_at,
            stations: Box::new(unlead95_stations),
        }),
        unlead98: RwLock::new(PriceList {
            petroleum_type: PETROLEUM_TYPE["UNLEAD_98"],
            updated_at,
            stations: Box::new(unlead98_stations),
        }),
        diesel_heat: RwLock::new(PriceList {
            petroleum_type: PETROLEUM_TYPE["DIESEL_HEAT"],
            updated_at,
            stations: Box::new(diesel_heat_stations),
        }),
        diesel_auto: RwLock::new(PriceList {
            petroleum_type: PETROLEUM_TYPE["DIESEL_AUTO"],
            updated_at,
            stations: Box::new(diesel_auto_stations),
        }),
        kerosene: RwLock::new(PriceList {
            petroleum_type: PETROLEUM_TYPE["KEROSENE"],
            updated_at,
            stations: Box::new(kerosene_stations),
        }),
    });

    info!("running app");

    HttpServer::new(move || {
        let konfig = config.clone();
        let token = Box::leak(konfig.secret.into_boxed_str());
        App::new()
            .app_data(data.clone())
            .service(unlead95)
            .service(unlead98)
            .service(diesel_heat)
            .service(diesel_auto)
            .service(kerosene)
            .service(
                web::resource("/prices/1/refresh")
                    .guard(guard::Header("X-TOKEN", token))
                    .route(web::patch().to(refresh_unlead95)),
            )
            .service(
                web::resource("/prices/2/refresh")
                    .guard(guard::Header("X-TOKEN", token))
                    .route(web::patch().to(refresh_unlead98)),
            )
            .service(
                web::resource("/prices/3/refresh")
                    .guard(guard::Header("X-TOKEN", token))
                    .route(web::patch().to(refresh_diesel_heat)),
            )
            .service(
                web::resource("/prices/4/refresh")
                    .guard(guard::Header("X-TOKEN", token))
                    .route(web::patch().to(refresh_diesel_auto)),
            )
            .service(
                web::resource("/prices/5/refresh")
                    .guard(guard::Header("X-TOKEN", token))
                    .route(web::patch().to(refresh_kerosene)),
            )
            .service(version)
    })
    .bind(address)?
    .run()
    .await
}
