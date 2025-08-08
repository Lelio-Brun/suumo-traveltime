use dioxus::prelude::*;
use reqwest::{Client, RequestBuilder};

use crate::Building;
use crate::Criterion;
use crate::Error;
use crate::backend;
use crate::components::BuildingView;
use crate::components::CriteriaForm;
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
pub fn List(app_id: String, api_key: String) -> Element {
    let url = "https://api.traveltimeapp.com/v4/geocoding/search";
    let geocode_client = Client::new();
    let geocode_request = geocode_client
        .get(url)
        .header("X-Application-Id", app_id.clone())
        .header("X-Api-Key", api_key.clone())
        .header("Accept-Language", "en-US");
    let geocode_request2 = geocode_request.try_clone().unwrap();

    let mut criteria_raw: Signal<Vec<Criterion>> = use_signal(|| vec![]);

    let _config: Resource<Result<(), Error>> = use_resource(move || async move {
        let criteria = backend::get_criteria().await?;
        criteria_raw.set(criteria);
        Ok(())
    });

    let criteria = use_resource(move || {
        let request = geocode_request.try_clone().unwrap();
        async move {
            let mut criteria = vec![];
            for criterion in criteria_raw() {
                let request = request.try_clone().unwrap();
                let location = geocode::geocode(&criterion.address, request).await?;
                criteria.push(Criterion {
                    location,
                    ..criterion
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
                            let _ = document::eval(&format!(r"initMap();")).await;
                        });
                    }

                    match &*scrape.read_unchecked() {
                        Some(Ok(buildings)) => {
                            spawn(async move {
                                let _ = document::eval(&format!(r"clearMap();")).await;
                            });

                            for criterion in criteria {
                                let (lng, lat) = criterion.location;
                                let lng = lng.clone();
                                let lat = lat.clone();
                                let color = criterion.color.clone();
                                spawn(async move {
                                    let _ = document::eval(&format!(
                                        r#"addDest({lng}, {lat}, "{color}");"#
                                    ))
                                    .await;
                                });
                            }

                            let buildings = buildings
                                .iter()
                                .filter(|building| building.times.len() == criteria.len());
                            for building in buildings {
                                let name = building.name.clone();
                                let lat = building.coordinates.0.clone();
                                let lng = building.coordinates.1.clone();
                                spawn(async move {
                                    let _ = document::eval(&format!(
                                        r#"addMarker("{name}", {lat}, {lng});"#
                                    ))
                                    .await;
                                });
                            }

                            spawn(async move {
                                let _ = document::eval(&r"fitMap();").await;
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
                    CriteriaForm { criteria_raw }

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
                            let _ = document::eval(&r"fitMap();").await;
                        }
                    }
              }
        }
    }
}
