use dioxus::prelude::*;
use dioxus_logger::tracing;
use reqwest::{Client, RequestBuilder};

use crate::ADDRESS;
use crate::Building;
use crate::Criterion;
use crate::Error;
use crate::TIMEOUT;
use crate::TransportationMode;
use crate::components::BuildingView;
use crate::geocode;
use crate::scrape;

async fn scrape_job<'a>(
    criteria: Resource<Result<Vec<Criterion>, Error>>,
    geocode_request: RequestBuilder,
    app_id: &'a str,
    api_key: &'a str,
) -> Result<Vec<Building>, Error> {
    let mut buildings = scrape::scrape(geocode_request).await?;

    match (*criteria.read_unchecked()).as_ref() {
        Some(Ok(criteria)) => {
            geocode::get_travel_time(app_id, api_key, &mut buildings, criteria).await
        }
        Some(Err(e)) => Err(Error::Misc(e.to_string())), // TODO
        None => Err(Error::Misc("Pending destination calculation".to_string())),
    }?;

    Ok(buildings)
}

#[component]
fn Criteria(criteria_raw: Signal<Vec<(TransportationMode, String, usize)>>) -> Element {
    rsx! {
        div {
            id: "criteria",
            label { for: "address", "Address" }
            label { for: "mode", "Mode" }
            label { for: "time", "Time" }
            for (k, (mode, address, time)) in criteria_raw().iter().enumerate() {
                input {
                    r#type: "search",
                    name: "address{k}",
                    value: address.as_str(),
                }
                select {
                    name: "mode{k}",
                    option {
                        value: "cycling",
                        selected: *mode == TransportationMode::Cycling,
                        "Bicycle"
                    }
                    option {
                        value: "public",
                        selected: *mode == TransportationMode::Public,
                        "Public transportation"
                    }
                    option {
                        value: "driving",
                        selected: *mode == TransportationMode::Driving,
                        "Driving"
                    }
                    option {
                        value: "walking",
                        selected: *mode == TransportationMode::Walking,
                        "Walking"
                    }
                }
                input {
                    class: "time",
                    name: "time{k}",
                    r#type: "number",
                    value: *time,
                    min: "1",
                    max: "999"
                }
            }
        }
    }
}

#[component]
pub fn List(app_id: String, api_key: String) -> Element {
    let url = "https://api.traveltimeapp.com/v4/geocoding/search";
    let geocode_client = Client::new();
    let geocode_request = geocode_client
        .get(url)
        .header("X-Application-Id", app_id.clone())
        .header("X-Api-Key", api_key.clone())
        .header("Accept-Language", "en-US");
    let geocode_request2 = geocode_request.try_clone().unwrap();

    let mut criteria_count = use_signal(|| 1);

    let mut criteria_raw =
        use_signal(|| vec![(TransportationMode::Cycling, ADDRESS.to_string(), TIMEOUT)]);

    let criteria = use_resource(move || {
        let request = geocode_request.try_clone().unwrap();
        async move {
            let mut criteria = vec![];
            for (mode, address, time) in criteria_raw() {
                let request = request.try_clone().unwrap();
                let location = geocode::geocode(&address, request).await?;
                criteria.push(Criterion {
                    mode,
                    time,
                    location,
                });
            }
            Ok(criteria)
        }
    });

    let scrape = use_resource(move || {
        let api_key = api_key.clone();
        let app_id = app_id.clone();
        let request = geocode_request2.try_clone().unwrap();
        async move { scrape_job(criteria, request, &app_id, &api_key).await }
    });

    let mut mounted_map: Signal<bool> = use_signal(|| false);
    let mut initialized_map = false;
    use_effect(move || {
        if mounted_map() {
            match &*criteria.read_unchecked() {
                Some(Ok(criteria)) => {
                    if !initialized_map {
                        initialized_map = true;
                        spawn(async move {
                            let e = document::eval(&format!(r"initMap();")).await;
                        });
                    }

                    match &*scrape.read_unchecked() {
                        Some(Ok(buildings)) => {
                            for criterion in criteria {
                                let (lng, lat) = criterion.location;
                                let lng = lng.clone();
                                let lat = lat.clone();
                                spawn(async move {
                                    let e = document::eval(&format!(
                                        r"clearMap(); addDest({lng}, {lat});"
                                    ))
                                    .await;
                                });
                            }

                            let buildings = buildings
                                .iter()
                                .filter(|building| !building.times.is_empty());
                            for building in buildings {
                                let name = building.name.clone();
                                let lat = building.coordinates.0.clone();
                                let lng = building.coordinates.1.clone();
                                spawn(async move {
                                    let e = document::eval(&format!(
                                        r#"addMarker("{name}", {lat}, {lng});"#
                                    ))
                                    .await;
                                });
                            }

                            spawn(async move {
                                let e = document::eval(&r"fitMap();").await;
                            });
                        }
                        _ => (),
                    }
                }
                _ => (),
            };
        }
    });

    rsx! {
        div { id: "view",
              div { id: "ui",
                    form { id: "criteria_form",
                           onsubmit: move |event| async move {
                               let values = event.values();
                               let mut criteria = vec![];
                               for k in 0..criteria_count() {
                                   let address = values.get(format!("address{k}").as_str()).unwrap().as_value();
                                   let mode = match
                                       values.get(format!("mode{k}").as_str()).unwrap().as_value().as_str() {
                                           "cycling" => Ok(TransportationMode::Cycling),
                                           "driving" => Ok(TransportationMode::Driving),
                                           "walking" => Ok(TransportationMode::Walking),
                                           "public" => Ok(TransportationMode::Public),
                                           _ => Err(Error::Misc("unknown transportation mode".to_string()))
                                       }?;
                                   let time = values.get(format!("time{k}").as_str()).unwrap().as_value().parse::<usize>()?;

                                   criteria.push((mode, address, time))
                               }
                               Ok(criteria_raw.set(criteria))
                           },
                           Criteria { criteria_raw }
                           input {
                               id: "add_criterion",
                               r#type: "button",
                               value: "Add",
                               onclick: move |_| async move {

                               }
                           }
                           input {
                               id: "submit_search",
                               r#type: "submit",
                               value: "Ok"
                           }
                    }

                    match &*scrape.read_unchecked() {
                        Some(Ok(buildings)) => {
                            let buildings = buildings.into_iter().cloned().filter(|building| building.times.len() == criteria_raw().len());
                            let bui_count = buildings.clone().count();
                            let apt_count = buildings.clone().fold(0, |count, building| count + building.apartments.len());

                            rsx! {
                                div {
                                    "Listing {apt_count} apartments in {bui_count} buildings:"
                                }
                                ul { id: "buildings",
                                     for building in buildings {
                                         BuildingView { building }
                                     }
                                }
                            }
                        }
                        Some(Err(err)) => rsx! { "{err}" },
                        None => rsx! { "Loading..." },
                    }
              }
              div { id: "map",
                    onmounted: move |_| {
                        mounted_map.set(true);
                    },
                    onresize: move |_| {
                        async move {
                            let e = document::eval(&r"fitMap();").await;
                        }
                    }
              }
        }
    }
}
