use dioxus::prelude::*;
use dioxus_logger::tracing;
use reqwest::RequestBuilder;

use crate::{Building, Criterion, Error, SUUMOURL, TransportationMode, backend, geocode, scrape};

use super::ClonableRequestBuilder;

#[component]
fn Criteria(criteria_raw: Signal<Vec<Criterion>>) -> Element {
    let criteria = criteria_raw();
    let multiple = criteria.len() > 1;

    rsx! {
        div {
            id: "criteria",
            span {  }
            span {  }
            label { for: "address", "Address" }
            label { for: "mode", "Mode" }
            label { for: "time", "Time" }
            for (k, criterion) in criteria.into_iter().enumerate() {
                if multiple {
                    button {
                        id: "rem_criterion",
                        r#type: "button",
                        onclick: move |_| {
                            criteria_raw.remove(k);
                        },
                        i { class: "fa-solid fa-circle-minus fa-lg"}
                    }
                } else {
                    span {  }
                }
                div {
                    "style": "width: 15px; height: 15px; background: {criterion.color}99; border: 2px solid {criterion.color}; border-radius: 50%"
                }
                input {
                    r#type: "search",
                    name: "address{k}",
                    key: "address{k}",
                    value: criterion.address
                }
                select {
                    name: "mode{k}",
                    key: "mode{k}",
                    option {
                        value: "cycling",
                        selected: criterion.mode == TransportationMode::Cycling,
                        "Bicycle"
                    }
                    option {
                        value: "public",
                        selected: criterion.mode == TransportationMode::Public,
                        "Public transportation"
                    }
                    option {
                        value: "driving",
                        selected: criterion.mode == TransportationMode::Driving,
                        "Driving"
                    }
                    option {
                        value: "walking",
                        selected: criterion.mode == TransportationMode::Walking,
                        "Walking"
                    }
                }
                input {
                    class: "time",
                    name: "time{k}",
                    key: "time{k}",
                    r#type: "number",
                    value: criterion.time,
                    min: "1",
                    max: "999"
                }

            }
        }
    }
}

#[component]
pub fn CriteriaForm(
    app_id: String,
    api_key: String,
    geocode_request: ClonableRequestBuilder,
    criteria_raw: Signal<Vec<Criterion>>,
    criteria_located: Signal<Vec<Criterion>>,
    buildings: Signal<Vec<Building>>,
    scrape_progress: Signal<f64>,
) -> Element {
    let suumo_url = use_server_future(backend::get_suumo_url)?;
    let mut suumo_url_sig = use_signal(|| SUUMOURL.to_string());

    match &*suumo_url.read_unchecked() {
        None => rsx! { div { "Checking database..." } },
        Some(Ok(_suumo_url)) => {
            // tracing::debug!("{suumo_url}");
            rsx! {
                form { id: "criteria_form",
                       onsubmit: move |event| {
                           // TODO: ErrorBoundary to handle errors
                           let api_key = api_key.clone();
                           let app_id = app_id.clone();
                           let request = geocode_request.clone().0;
                           async move {
                               let values = event.values();
                               let mut criteria = vec![];
                               for (k, criterion) in criteria_raw().into_iter().enumerate() {
                                   let address = values.get(format!("address{k}").as_str()).unwrap().as_value();
                                   let mode = match
                                       values.get(format!("mode{k}").as_str()).unwrap().as_value().as_str() {
                                           "cycling" => Ok(TransportationMode::Cycling),
                                           "driving" => Ok(TransportationMode::Driving),
                                           "walking" => Ok(TransportationMode::Walking),
                                           "public" => Ok(TransportationMode::Public),
                                           _ => Err(Error::Misc("unknown transportation mode".to_string()))
                                       }?;
                                   let time = values.get(format!("time{k}").as_str()).unwrap().as_value();

                                   if !address.is_empty() && !time.is_empty() {
                                       let time = time.parse::<usize>()?;
                                       criteria.push(Criterion { mode, address, time, ..criterion })
                                   }
                               }

                               _ = backend::set_criteria(criteria.clone()).await;
                               _ = backend::set_suumo_url(suumo_url_sig()).await;

                               criteria_raw.set(criteria.clone());

                               let mut criteria_loc = vec![];
                               for criterion in criteria {
                                   let request = request.try_clone().unwrap();
                                   let location = geocode::geocode(&criterion.address, request).await?;
                                   criteria_loc.push(Criterion {
                                       location,
                                       ..criterion
                                   });
                               }
                               criteria_located.set(criteria_loc.clone());

                               let mut buildings_v = scrape::scrape(scrape_progress, request).await?;
                               geocode::get_travel_time(&app_id, &api_key, &mut buildings_v, &criteria_loc)
                                   .await?;
                               buildings.set(buildings_v);

                               Ok(())
                           }
                       },
                       div { id: "suumo",
                             label { for: "suumo_url", "Suumo URL" }
                             input {
                                 r#type: "url",
                                 name: "suumo_url",
                                 key: "suumo_url",
                                 value: "{suumo_url_sig}",
                                 oninput: move |event| suumo_url_sig.set(event.value())
                             }
                       }
                       Criteria { criteria_raw }
                       button {
                           id: "add_criterion",
                           r#type: "button",
                           onclick: move |_| {
                               let last = criteria_raw().last().unwrap().clone();
                               criteria_raw.push(Criterion { color: random_color::RandomColor::new().to_hex(), ..last })
                           },
                           i { class: "fa-solid fa-circle-plus fa-lg"}
                       }
                       button {
                           id: "submit_search",
                           r#type: "submit",
                           i { class: "fa-solid fa-magnifying-glass fa-lg"}
                           " Search"
                       }
                }

            }
        }
        Some(Err(_)) => rsx! {},
    }
}
