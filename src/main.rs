use dioxus::prelude::*;
use scraper::error::SelectorErrorKind;
use thiserror::Error;

use std::{collections::HashMap, num::ParseIntError};

mod backend;
mod components;
mod geocode;
mod scrape;

use crate::components::*;

#[derive(Error, Debug)]
pub enum Error {
    #[error("server error: {0}")]
    ServerError(ServerFnError),
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("serde_json error: {0}")]
    SerdeJSON(#[from] serde_json::Error),
    #[error("parse error: {0}")]
    ParseInt(#[from] ParseIntError),
    #[error("selector error: {0}")]
    Scrape(String),
    #[error("misc error: {0}")]
    Misc(String),
}

impl<'a> From<SelectorErrorKind<'a>> for Error {
    fn from(error: SelectorErrorKind<'a>) -> Self {
        Self::Scrape(error.to_string())
    }
}

impl From<ServerFnError> for Error {
    fn from(error: ServerFnError) -> Self {
        Self::ServerError(error)
    }
}

const MAIN_CSS: Asset = asset!("/assets/main.css");
const MAP_JS: Asset = asset!("/assets/map.js", JsAssetOptions::new().with_minify(false));

#[derive(Clone, PartialEq)]
struct Apartment {
    rent: String,
    fees: Option<String>,
    id: u64,
    deposit: Option<String>,
    key_money: Option<String>,
    kind: String,
    area: String,
    plan: String,
    url: String,
}

#[derive(Clone, PartialEq)]
struct Building {
    name: String,
    address: String,
    coordinates: (f64, f64),
    times: HashMap<usize, (Criterion, usize)>,
    apartments: Vec<Apartment>,
}

#[derive(Clone, PartialEq, Debug)]
enum TransportationMode {
    Cycling,
    Driving,
    Walking,
    Public,
}

#[derive(Clone, PartialEq)]
struct Criterion {
    mode: TransportationMode,
    location: (f64, f64),
    time: usize,
    color: String,
}

const ADDRESS: &str = "東京都渋谷区渋谷1-3-7";
const TIMEOUT: usize = 20;
const DESTCOLOR: &str = "#c92a2a";

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut credentials = use_server_future(backend::get_credentials)?;

    rsx! {
        document::Stylesheet {
            href: MAIN_CSS
        }
        document::Link {
            rel: "stylesheet",
            href: "https://unpkg.com/leaflet@1.9.4/dist/leaflet.css",
            integrity: "sha256-p4NxAoJBhIIN+hmNHrzRCf9tD/miZyoHS5obTRR9BMY=",
            crossorigin: ""
        }
        document::Script {
            src: "https://kit.fontawesome.com/546761b7ee.js",
            crossorigin: "anonymous"
        }
        document::Script {
            src: "https://unpkg.com/leaflet@1.9.4/dist/leaflet.js",
            integrity: "sha256-20nQCchB9co0qIjJZRGuk2/Z9VM+kNiyxNV1lvTlZBo=",
            crossorigin: ""
        }
        document::Script {
            src: MAP_JS
        }

        match &*credentials.read_unchecked() {
            None => rsx! { div { "Checking database..." } },
            Some(Ok((app_id, api_key))) => rsx! {
                List { app_id: app_id, api_key: api_key }
            },
            Some(Err(_)) => rsx! {
                form { onsubmit: move |event| async move {
                    let values = event.values();
                    _ = backend::save_credentials(values.get("app_id").unwrap().as_value(), values.get("api_key").unwrap().as_value()).await;
                    credentials.restart();
                },
                       input { name: "app_id", placeholder: "App ID" }
                       input { name: "api_key", placeholder: "API Key" }
                       input { r#type: "submit", value: "Ok" }
                }
            }
        }

    }
}
