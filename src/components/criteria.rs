use dioxus::prelude::*;

use crate::{Error, TransportationMode};

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
pub fn CriteriaForm(
    criteria_count: Signal<usize>,
    criteria_raw: Signal<Vec<(TransportationMode, String, usize)>>,
) -> Element {
    rsx! {
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

    }
}
