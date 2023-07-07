use actix_web::body::BoxBody;
use actix_web::{get, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use cygaz_lib::{fetch_prices, PetroleumStation, PetroleumType};
use log::{debug, info, warn};
use reqwest::header::HeaderMap;
use reqwest::{Error, Response};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

#[derive(Clone, Serialize)]
struct PriceList {
    updated_at: u128,
    petroleum_type: PetroleumType,
    stations: Vec<PetroleumStation>,
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
    unlead95: PriceList,
    unlead98: PriceList,
    diesel_heat: PriceList,
    diesel_auto: PriceList,
    kerosene: PriceList,
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

fn refresh_prices(
    prices: web::Data<Arc<RwLock<AppStateWithPrices>>>
) {
    debug!("refreshing prices");

    let unlead95_handler = thread::spawn(|| {
        debug!("warming up unlead 95");
        fetch_prices(PetroleumType::Unlead95).unwrap_or_default()
    });

    let unlead98_handler = thread::spawn(|| {
        debug!("warming up unlead 98");
        fetch_prices(PetroleumType::Unlead98).unwrap_or_default()
    });

    let diesel_heat_handler = thread::spawn(|| {
        debug!("warming up diesel heat");
        fetch_prices(PetroleumType::DieselHeat).unwrap_or_default()
    });

    let diesel_auto_handler = thread::spawn(|| {
        debug!("warming up diesel auto");
        fetch_prices(PetroleumType::DieselAuto).unwrap_or_default()
    });

    let kerosene_handler = thread::spawn(|| {
        debug!("warming up kerosene");
        fetch_prices(PetroleumType::Kerosene).unwrap_or_default()
    });

    let unlead95_stations = unlead95_handler.join().unwrap_or_default();
    let unlead98_stations = unlead98_handler.join().unwrap_or_default();
    let diesel_heat_stations = diesel_heat_handler.join().unwrap_or_default();
    let diesel_auto_stations = diesel_auto_handler.join().unwrap_or_default();
    let kerosene_stations = kerosene_handler.join().unwrap_or_default();

    // fetch timestamp
    let epoch = SystemTime::now().duration_since(UNIX_EPOCH);
    let epoch_updated_at = epoch.unwrap().as_millis();

    let mut lock = prices.write().unwrap();

    lock.unlead95 = PriceList {
        petroleum_type: PetroleumType::Unlead95,
        updated_at: epoch_updated_at,
        stations: unlead95_stations,
    };

    lock.unlead98 = PriceList {
        petroleum_type: PetroleumType::Unlead98,
        updated_at: epoch_updated_at,
        stations: unlead98_stations,
    };

    lock.diesel_heat = PriceList {
        petroleum_type: PetroleumType::DieselHeat,
        updated_at: epoch_updated_at,
        stations: diesel_heat_stations,
    };

    lock.diesel_auto = PriceList {
        petroleum_type: PetroleumType::DieselAuto,
        updated_at: epoch_updated_at,
        stations: diesel_auto_stations,
    };

    lock.kerosene = PriceList {
        petroleum_type: PetroleumType::Kerosene,
        updated_at: epoch_updated_at,
        stations: kerosene_stations,
    };
}

#[get("/prices/1")]
async fn unlead95(data: web::Data<Arc<RwLock<AppStateWithPrices>>>) -> impl Responder {
    let state = data.read().unwrap();
    state.unlead95.clone()
}

#[get("/prices/2")]
async fn unlead98(data: web::Data<Arc<RwLock<AppStateWithPrices>>>) -> impl Responder {
    let state = data.read().unwrap();
    state.unlead98.clone()
}

#[get("/prices/3")]
async fn diesel_heat(data: web::Data<Arc<RwLock<AppStateWithPrices>>>) -> impl Responder {
    let state = data.read().unwrap();
    state.diesel_heat.clone()
}

#[get("/prices/4")]
async fn diesel_auto(data: web::Data<Arc<RwLock<AppStateWithPrices>>>) -> impl Responder {
    let state = data.read().unwrap();
    state.diesel_auto.clone()
}

#[get("/prices/5")]
async fn kerosene(data: web::Data<Arc<RwLock<AppStateWithPrices>>>) -> impl Responder {
    let state = data.read().unwrap();
    state.kerosene.clone()
}

#[get("/version")]
async fn version() -> impl Responder {
    env!("CARGO_PKG_VERSION")
}

async fn refresh_petroleum_type(
    config: Arc<Config>,
    petroleum_type: PetroleumType,
) -> Result<Response, Error> {
    let endpoint = format!(
        "http://{}:{}/prices/{}/refresh",
        config.host, config.port, petroleum_type as i32
    );

    info!("calling {}", endpoint);

    let mut headers = HeaderMap::new();
    headers.insert("X-TOKEN", config.secret.parse().unwrap());

    let client = reqwest::Client::new();
    client.patch(endpoint).headers(headers).send().await
}

async fn setup_cron(config: Arc<Config>, prices: web::Data<Arc<RwLock<AppStateWithPrices>>>) -> JobScheduler {
    debug!("setting up cron");

    let sched = JobScheduler::new().await.unwrap();

    if let Err(e) = sched.add(
        Job::new_async("0 1,16,31,46 * * * *", move |_uuid, _l| {
            let config = config.clone();
            let prices = prices.clone();

            Box::pin(async move {
                if let Err(e) =
                    refresh_petroleum_type(config.clone(), PetroleumType::Unlead95).await
                {
                    warn!("error refreshing unlead95 {}", e);
                }
                if let Err(e) =
                    refresh_petroleum_type(config.clone(), PetroleumType::Unlead98).await
                {
                    warn!("error refreshing unlead98 {}", e);
                }
                if let Err(e) =
                    refresh_petroleum_type(config.clone(), PetroleumType::DieselHeat).await
                {
                    warn!("error refreshing diesel heat {}", e);
                }
                if let Err(e) =
                    refresh_petroleum_type(config.clone(), PetroleumType::DieselAuto).await
                {
                    warn!("error refreshing diesel auto {}", e);
                }
                if let Err(e) =
                    refresh_petroleum_type(config.clone(), PetroleumType::Kerosene).await
                {
                    warn!("error refreshing kerosene {}", e);
                }

                refresh_prices(prices);

                info!("scheduler finished successfully");
            })
        })
        .unwrap(),
    ).await {
        warn!("error scheduling {:?}", e);
    }

    sched
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let raw = envy::from_env::<Config>().unwrap();
    let config = Arc::new(raw);
    let address = format!("{}:{}", config.host, config.port);

    let epoch = SystemTime::now().duration_since(UNIX_EPOCH);
    let updated_at = epoch.unwrap().as_millis();

    info!("warming up initial cache");

    let data = web::Data::new(Arc::new(RwLock::new(AppStateWithPrices {
        unlead95: PriceList {
            petroleum_type: PetroleumType::Unlead95,
            updated_at,
            stations: vec![],
        },
        unlead98: PriceList {
            petroleum_type: PetroleumType::Unlead98,
            updated_at,
            stations: vec![],
        },
        diesel_heat: PriceList {
            petroleum_type: PetroleumType::DieselHeat,
            updated_at,
            stations: vec![],
        },
        diesel_auto: PriceList {
            petroleum_type: PetroleumType::DieselAuto,
            updated_at,
            stations: vec![],
        },
        kerosene: PriceList {
            petroleum_type: PetroleumType::Kerosene,
            updated_at,
            stations: vec![],
        },
    })));

    refresh_prices(data.clone());

    let scheduler = setup_cron(config.clone(), data.clone());

    if let Err(e) = scheduler.await.start().await {
        warn!("failed to start scheduler {:?}", e);
    }

    info!("starting http server @ {}", address.clone());

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
        .bind(address)
        .unwrap()
        .run()
        .await.expect("server failed to start")
}
