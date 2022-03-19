use std::sync::{LockResult, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};
use actix_web::{get, App, HttpResponse, HttpServer, Responder, HttpRequest, web};
use actix_web::body::BoxBody;
use cygaz_lib::{fetch_prices, PETROLEUM_TYPE, PetroleumStation};
use serde::Serialize;
use log::{info, debug};

#[derive(Clone, Serialize)]
struct PriceList {
    updated_at: u128,
    petroleum_type: u32,
    stations: Box<Vec<PetroleumStation>>,
}

struct AppStateWithPrices {
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

async fn fetch_petroleum(petroleum_type: u32, lock: LockResult<MutexGuard<'_, PriceList>>) -> PriceList {
    let epoch = SystemTime::now().duration_since(UNIX_EPOCH);
    let epoch_updated_at = epoch.unwrap().as_millis();
    let mut petroleum_list = lock.unwrap();
    let petr_stations = petroleum_list.stations.clone();
    let petr_updated_at = petroleum_list.updated_at;

    if epoch_updated_at - petr_updated_at > 600000 { // over 10 minutes
        debug!("fetching {} {}-{}={}",
            petroleum_type, epoch_updated_at, petr_updated_at, epoch_updated_at - petr_updated_at);
        let stations = fetch_prices(petroleum_type).await;
        debug!("found {} station/prices for {}", stations.len(), petroleum_type);
        *petroleum_list = PriceList { petroleum_type, updated_at: epoch_updated_at, stations: Box::new(stations.clone()) };
        return PriceList { petroleum_type, updated_at: epoch_updated_at, stations: Box::new(stations) };
    }

    return PriceList{ petroleum_type, updated_at: petr_updated_at, stations: petr_stations };
}

#[get("/prices/1")]
async fn unlead95(data: web::Data<AppStateWithPrices>) -> impl Responder {
    fetch_petroleum(PETROLEUM_TYPE["UNLEAD_95"], data.unlead95.lock()).await
}

#[get("/prices/2")]
async fn unlead98(data: web::Data<AppStateWithPrices>) -> impl Responder {
    fetch_petroleum(PETROLEUM_TYPE["UNLEAD_98"], data.unlead98.lock()).await
}

#[get("/prices/3")]
async fn diesel_heat(data: web::Data<AppStateWithPrices>) -> impl Responder {
    fetch_petroleum(PETROLEUM_TYPE["DIESEL_HEAT"], data.diesel_heat.lock()).await
}

#[get("/prices/4")]
async fn diesel_auto(data: web::Data<AppStateWithPrices>) -> impl Responder {
    fetch_petroleum(PETROLEUM_TYPE["DIESEL_AUTO"], data.diesel_auto.lock()).await
}

#[get("/prices/5")]
async fn kerosene(data: web::Data<AppStateWithPrices>) -> impl Responder {
    fetch_petroleum(PETROLEUM_TYPE["KEROSENE"], data.kerosene.lock()).await
}

#[get("/version")]
async fn version() -> impl Responder {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    VERSION
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let epoch = SystemTime::now().duration_since(UNIX_EPOCH);
    let updated_at = epoch.unwrap().as_millis();
    info!("warming up cache");

    debug!("warming up unlead 95");
    let unlead95_stations = fetch_prices(PETROLEUM_TYPE["UNLEAD_95"]).await;
    debug!("unlead 95 cached");

    debug!("warming up unlead 98");
    let unlead98_stations = fetch_prices(PETROLEUM_TYPE["UNLEAD_98"]).await;
    debug!("unlead 98 cached");

    debug!("warming up diesel heat");
    let diesel_heat_stations = fetch_prices(PETROLEUM_TYPE["DIESEL_HEAT"]).await;
    debug!("diesel heat cached");

    debug!("warming up diesel auto");
    let diesel_auto_stations = fetch_prices(PETROLEUM_TYPE["DIESEL_AUTO"]).await;
    debug!("diesel auto cached");

    debug!("warming up kerosene");
    let kerosene_stations = fetch_prices(PETROLEUM_TYPE["KEROSENE"]).await;
    debug!("kerosene cached");

    let data = web::Data::new(AppStateWithPrices{
        unlead95: Mutex::new(PriceList {
            petroleum_type: PETROLEUM_TYPE["UNLEAD_95"],
            updated_at,
            stations: Box::new(unlead95_stations),
        }),
        unlead98: Mutex::new(PriceList {
            petroleum_type: PETROLEUM_TYPE["UNLEAD_98"],
            updated_at,
            stations: Box::new(unlead98_stations),
        }),
        diesel_heat: Mutex::new(PriceList {
            petroleum_type: PETROLEUM_TYPE["DIESEL_HEAT"],
            updated_at,
            stations: Box::new(diesel_heat_stations),
        }),
        diesel_auto: Mutex::new(PriceList {
            petroleum_type: PETROLEUM_TYPE["DIESEL_AUTO"],
            updated_at,
            stations: Box::new(diesel_auto_stations),
        }),
        kerosene: Mutex::new(PriceList {
            petroleum_type: PETROLEUM_TYPE["KEROSENE"],
            updated_at,
            stations: Box::new(kerosene_stations),
        }),
    });

    info!("running at port 8080");

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
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}