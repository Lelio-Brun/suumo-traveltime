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

pub struct ClonableRequestBuilder(pub RequestBuilder);

impl Clone for ClonableRequestBuilder {
    fn clone(&self) -> Self {
        ClonableRequestBuilder(self.0.try_clone().unwrap())
    }
}

impl PartialEq for ClonableRequestBuilder {
    fn eq(&self, _other: &Self) -> bool {
        false
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
    let geocode_request = ClonableRequestBuilder(geocode_request);

    let mut criteria_raw: Signal<Vec<Criterion>> = use_signal(|| vec![]);
    let criteria_located: Signal<Vec<Criterion>> = use_signal(|| vec![]);

    let _config: Resource<Result<(), Error>> = use_resource(move || async move {
        let criteria = backend::get_criteria().await?;
        criteria_raw.set(criteria);
        Ok(())
    });

    let buildings: Signal<Vec<Building>> = use_signal(|| vec![]);

    let scrape_progress: Signal<f64> = use_signal(|| 0.0);

    let mut mounted_map: Signal<bool> = use_signal(|| false);
    let mut initialized_map = false;
    use_effect(move || {
        if mounted_map() {
            if !initialized_map {
                initialized_map = true;
                spawn(async move {
                    let _ = document::eval(&format!(r"initMap();")).await;
                });
            }

            spawn(async move {
                let _ = document::eval(&format!(r"clearMap();")).await;
            });

            let criteria = criteria_located();
            for criterion in &criteria {
                let (lng, lat) = criterion.location;
                let lng = lng.clone();
                let lat = lat.clone();
                let color = criterion.color.clone();
                spawn(async move {
                    let _ = document::eval(&format!(r#"addDest({lng}, {lat}, "{color}");"#)).await;
                });
            }

            let buildings = buildings();
            let buildings = buildings
                .iter()
                .filter(|building| building.times.len() == criteria.len());
            for building in buildings {
                let name = building.name.clone();
                let lat = building.coordinates.0.clone();
                let lng = building.coordinates.1.clone();
                spawn(async move {
                    let _ = document::eval(&format!(r#"addMarker("{name}", {lat}, {lng});"#)).await;
                });
            }

            spawn(async move {
                let _ = document::eval(&r"fitMap();").await;
            });
        }
    });

    rsx! {
        div { id: "view",
              div { id: "ui",
                    CriteriaForm {
                        app_id,
                        api_key,
                        geocode_request,
                        criteria_raw,
                        criteria_located,
                        buildings,
                        scrape_progress
                    }

                    {
                        let buildings = buildings().into_iter().filter(|building| building.times.len() == criteria_located().len());
                        let bui_count = buildings.clone().count();
                        let apt_count = buildings.clone().fold(0, |count, building| count + building.apartments.len());

                        rsx! {
                            div {
                                "Listing {apt_count} apartments in {bui_count} buildings ({scrape_progress()}):"
                            }
                            ul { id: "buildings",
                                 for building in buildings {
                                     BuildingView { building }
                                 }
                            }
                        }
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
