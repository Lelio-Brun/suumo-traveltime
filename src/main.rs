use dioxus::prelude::*;
use dioxus_logger::*;
use reqwest::{Client, RequestBuilder};
use scraper::{Html, Selector};
use serde_json::json;
use serde_json::Value;
use serde_json::Map;

use std::collections::HashMap;
use std::error;

// const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
// const HEADER_SVG: Asset = asset!("/assets/header.svg");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut credentials = use_server_future(get_credentials)?;


    rsx! {
        // document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        match &*credentials.read_unchecked() {
            None => rsx! { div { "Checking database..." } },
            Some(Ok((app_id, api_key))) => rsx! {
                List { app_id: app_id, api_key: api_key }
            },
            Some(Err(_)) => rsx! {
                form { onsubmit: move |event| async move {
                    let values = event.values();
                    _ = save_credentials(values.get("app_id").unwrap().as_value(), values.get("api_key").unwrap().as_value()).await;
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

#[derive(Clone)]
struct Apartment {
    rent: String,
    fees: Option<String>,
    // floor: String,
    deposit: Option<String>,
    key_money: Option<String>,
    kind: String,
    area: String,
    plan: String,
}

#[derive(Clone)]
struct Building {
    name: String,
    coordinates: (f64, f64),
    time: Option<usize>,
    apartments: Vec<Apartment>
}

const ADDRESS: &str = "東京都渋谷区渋谷1-3-7";
const TIMEOUT: usize = 1200;

#[cfg(feature = "server")]
thread_local! {
    pub static DB: rusqlite::Connection = {
        let conn = rusqlite::Connection::open("data.db").expect("Failed to open database");

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS buildings (
                address TEXT PRIMARY KEY,
                lat REAL,
                lng REAL,
                reachable INTEGER,
                time INTEGER
            );
            CREATE TABLE IF NOT EXISTS credentials (
                app_id TEXT NOT NULL,
                key TEXT NOT NULL
            );").unwrap();

        conn
    };
}

#[server]
async fn save_credentials(app_id: String, api_key: String) -> Result<(), ServerFnError> {
    DB.with(|db| db.execute("DELETE FROM credentials", []))?;
    DB.with(|db| db.execute("INSERT INTO credentials VALUES (?1, ?2)", (app_id, api_key)))?;
    Ok(())
}

#[server]
async fn get_credentials() -> Result<(String, String), ServerFnError> {
    Ok(DB.with(|db| db.query_row("SELECT * FROM credentials",
                               [],
                               |row| {
                                   let app_id: String = row.get(0)?;
                                   let api_key: String = row.get(1)?;
                                   Ok ((app_id, api_key))
                               }
    ))?)
}

#[server]
async fn get_coords(address: String) -> Result<(f64, f64), ServerFnError> {
    Ok(DB.with(|db| db.query_row("SELECT lat, lng FROM buildings WHERE address = ?1",
                               [address],
                               |row| {
                                   let lat: f64 = row.get(0)?;
                                   let lng: f64 = row.get(1)?;
                                   Ok ((lng, lat))
                               }
    ))?)
}

#[server]
async fn set_coords(address: String, lng: f64, lat: f64) -> Result<(), ServerFnError> {
    DB.with(|db| db.execute("INSERT INTO buildings VALUES (?1, ?2, ?3, NULL, NULL) ON CONFLICT DO NOTHING",
                            (address, lat, lng)))?;
    Ok(())
}

async fn geocode<'a>(address: &'a str, request: RequestBuilder) -> Result<(f64, f64), Box<dyn error::Error>> {
    match get_coords(address.to_string()).await {
        Err(_) => {
            let text = request
                .query(&[("query", address)])
                .send()
                .await?
                .text()
                .await?;
            let json: serde_json::Value = serde_json::from_str(&text)?;

            let coordinates = &json["features"][0]["geometry"]["coordinates"];
            let lng = coordinates[0].as_f64().unwrap();
            let lat = coordinates[1].as_f64().unwrap();

            set_coords(address.to_string(), lng, lat).await.unwrap();

            Ok((lng, lat))
        },
        Ok(ok) => Ok(ok)
    }
}

async fn get_travel_time<'a>(app_id: &'a str, api_key: &'a str, buildings: &mut Vec<Building>, destination: (f64, f64)) -> Result<(), Box<dyn error::Error>> {
    let mut table = HashMap::new();
    for (k, building) in buildings.iter_mut().enumerate() {
        table.insert(k + 1, building);
    }

    let mut locations: Vec<Value> = table.iter().map(|(k, building)| {
        let mut c = Map::new();
        c.insert("lng".to_owned(), building.coordinates.0.clone().into());
        c.insert("lat".to_owned(), building.coordinates.1.clone().into());

        let mut o = Map::new();
        o.insert("id".to_owned(), format!("{k}").into());
        o.insert("coords".to_owned(), c.into());
        o.into()
    }).collect();
    locations.push(json!({"id": "0", "coords": {"lng": destination.0, "lat": destination.1}}));

    let ids = (1..=table.len()).map(|k| format!("{k}")).collect::<Vec<String>>();

    let body = json!({
        "locations": locations,
        "arrival_searches": {
            "many_to_one": [
                {
                    "id": "suumo",
                    "arrival_location_id": "0",
                    "departure_location_ids": ids,
                    "travel_time": TIMEOUT,
                    "arrival_time_period": "weekday_morning",
                    "transportation": {
                        "type": "cycling+ferry"
                    },
                    "properties": [
                        "travel_time"
                    ],
                }
            ]
        }
    });

    let url = "https://api.traveltimeapp.com/v4/time-filter/fast";
    let client = Client::new();
    let text = client
        .post(url)
        .header("X-Application-Id", app_id)
        .header("X-Api-Key", api_key)
        .header("Content-Type", "application/json")
        .header("Accept-Language", "en-US")
        .json(&body)
        .send()
        .await?
        .text()
        .await?;
    let json: serde_json::Value = serde_json::from_str(&text)?;

    let locations = json["results"][0]["locations"].as_array().unwrap();

    for location in locations {
        let k: usize = location["id"].as_str().unwrap().parse()?;
        let time: usize = location["properties"]["travel_time"].as_i64().unwrap() as usize;
        table.entry(k).and_modify(|building| building.time = Some(time));
    }

    Ok(())
}

async fn scrape_job<'a>(app_id: &'a str, api_key: &'a str) -> Result<Vec<Building>, Box<dyn error::Error>> {
    let url = "https://suumo.jp/jj/chintai/ichiran/FR301FC001/?url=%2Fchintai%2Fichiran%2FFR301FC001%2F&ar=030&bs=040&pc=50&smk=&po1=25&po2=99&tc=0400501&tc=0400902&shkr1=03&shkr2=03&shkr3=03&shkr4=03&cb=0.0&ct=13.0&md=03&md=04&md=05&md=06&md=07&md=08&md=09&md=10&md=11&md=12&md=13&md=14&et=9999999&mb=25&mt=9999999&cn=9999999&ta=13&sc=13103&sc=13104&sc=13113&sc=13110&sc=13112";
    let url = format!("https://corsproxy.io/?url={url}");

    let client = Client::new();
    let request = client.get(url);

    let html = request.try_clone().unwrap()
                                  .send()
                                  .await?
                                  .text()
                                  .await?;

    let mut doc = Html::parse_document(&html);

    let pagination_sel = Selector::parse("ol.pagination-parts")?;
    let pages: usize = doc.select(&pagination_sel).next().ok_or("Pagination not found")?.child_elements().last().ok_or("Last page not found")?.text().collect::<String>().parse()?;

    let building_sel = Selector::parse("div.cassetteitem")?;
    let name_sel = Selector::parse("div.cassetteitem_content-title")?;
    let address_sel = Selector::parse("li.cassetteitem_detail-col1")?;
    let apartment_sel = Selector::parse("tr.js-cassette_link")?;
    let rent_sel = Selector::parse("span.cassetteitem_price--rent")?;
    let fees_sel = Selector::parse("span.cassetteitem_price--administration")?;
    let deposit_sel = Selector::parse("span.cassetteitem_price--deposit")?;
    let key_money_sel = Selector::parse("span.cassetteitem_price--gratuity")?;
    let kind_sel = Selector::parse("span.cassetteitem_madori")?;
    let area_sel = Selector::parse("span.cassetteitem_menseki")?;
    let plan_sel = Selector::parse("img.casssetteitem_other-thumbnail-img")?;

    let mut buildings = vec![];

    let url = "https://api.traveltimeapp.com/v4/geocoding/search";
    let geocode_client = Client::new();
    let geocode_request = geocode_client
        .get(url)
        .header("X-Application-Id", app_id)
        .header("X-Api-Key", api_key)
        .header("Accept-Language", "en-US");

    for page in 1..=pages {
        tracing::debug!("Page {page}");

        for building in doc.select(&building_sel) {
            let name: String = building.select(&name_sel).next().ok_or(format!("Title not found"))?.text().collect();
            let address: String = building.select(&address_sel).next().ok_or(format!("Address not found"))?.text().collect();

            let request = geocode_request.try_clone().unwrap();
            let coordinates = geocode(&address, request).await?;

            let mut apartments = vec![];

            for apartment in building.select(&apartment_sel) {
                let rent = apartment.select(&rent_sel).next().ok_or(format!("Rent not found. at {name}"))?.text().collect();
                let fees = apartment.select(&fees_sel).next().ok_or(format!("Fees not found. at {name}"))?.text().collect();
                let fees = if fees == "-" { None } else { Some(fees) };
                let deposit = apartment.select(&deposit_sel).next().ok_or(format!("Deposit not found. at {name}"))?.text().collect();
                let deposit = if deposit == "-" { None } else { Some(deposit) };
                let key_money = apartment.select(&key_money_sel).next().ok_or(format!("Key money not found. at {name}"))?.text().collect();
                let key_money = if key_money == "-" { None } else { Some(key_money) };
                let kind = apartment.select(&kind_sel).next().ok_or(format!("Kind not found. at {name}"))?.text().collect();
                let area = apartment.select(&area_sel).next().ok_or(format!("Area not found. at {name}"))?.text().collect();
                let plan = apartment.select(&plan_sel).next().ok_or(format!("Plan not found. at {name}"))?.attr("rel").ok_or("Rel not found")?.to_string();

                apartments.push(Apartment { rent, fees, deposit, key_money, kind, area, plan });
            }

            buildings.push(Building { name, coordinates, apartments, time: None });
        }

        let html = request.try_clone().unwrap().query(&[("page", format!("{page}"))])
                                               .send()
                                               .await?
                                               .text()
                                               .await?;

        doc = Html::parse_document(&html);
    }


    let destination = geocode(&ADDRESS, geocode_request).await?;
    get_travel_time(app_id, api_key, &mut buildings, destination).await?;

    Ok(buildings)
}

#[component]
pub fn List(app_id: String, api_key: String) -> Element {
    let scrape = use_resource(move || {
        let api_key = api_key.clone();
        let app_id = app_id.clone();
        async move { scrape_job(&app_id, &api_key).await }
    });

    rsx! {
        match &*scrape.read_unchecked() {
            Some(Ok(buildings)) =>
                {
                    let buildings = buildings.iter().filter(|building| building.time.is_some());
                    let count = buildings.clone().fold(0, |count, building| count + building.apartments.len());

                rsx! {
                    div {
                        "Listing {count} apartments:"
                    }
                    div {
                        for building in buildings {
                            div {
                                h3 { "{building.name}" }
                                h4 { "{(building.time.unwrap() as f32 / 60.0) as usize} min" }
                                for apartment in *building.apartments {
                                    div {
                                        img {
                                            src: "{apartment.plan}"
                                        }
                                        div {
                                            div { "{apartment.area} ({apartment.kind})" }
                                            div {
                                                match &apartment.fees {
                                                    Some(fees) => rsx! { "{apartment.rent} + {fees}" },
                                                    None => rsx! { "{apartment.rent}" },
                                                }
                                            }
                                            div {
                                                match (&apartment.deposit, &apartment.key_money) {
                                                    (Some(x), Some(y)) => rsx! { "{x} + {y}!" },
                                                    (Some(x), _) => rsx! { "{x}" },
                                                    (_, Some(x)) => rsx! { "{x}!" },
                                                    _ => rsx! { "No initial fees" },
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                }
            Some(Err(err)) => rsx! { "Error: {err:?}" },
            None => rsx! { "Loading..." },
        }
    }
}
