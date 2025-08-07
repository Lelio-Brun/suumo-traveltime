use dioxus::prelude::*;
// use dioxus_logger::tracing;

use crate::{Error, TransportationMode};

#[component]
fn Criteria(
    criteria_colors: Signal<Vec<String>>,
    criteria_raw: Signal<Vec<(TransportationMode, String, usize, String)>>,
) -> Element {
    let criteria = criteria_raw();
    let criteria_colors = criteria_colors();
    let mut default = (None, None, None);

    rsx! {
        div {
            id: "criteria",
            span {  }
            label { for: "address", "Address" }
            label { for: "mode", "Mode" }
            label { for: "time", "Time" }
            for (k, mode, address, time, color) in criteria_colors.into_iter().enumerate().map(|(k, color)| {
                let criterion = criteria.get(k);
                match criterion {
                    Some((mode, address, time, color)) => {
                        default = (Some(mode), Some(address), Some(time));
                        (k, default.0, default.1, default.2, color.clone())
                    }
                    None => (k, default.0, default.1, default.2, color)
                }
            }) {
                div {
                    "style": "width: 15px; height: 15px; background: {color}99; border: 2px solid {color}; border-radius: 50%"
                }
                input {
                    r#type: "search",
                    name: "address{k}",
                    key: "address{k}",
                    value: if let Some(address) = address { address.as_str() },
                    placeholder: if address.is_none() { "Address" },
                }
                select {
                    name: "mode{k}",
                    key: "mode{k}",
                    option {
                        value: "cycling",
                        selected: mode == Some(&TransportationMode::Cycling),
                        "Bicycle"
                    }
                    option {
                        value: "public",
                        selected: mode == Some(&TransportationMode::Public),
                        "Public transportation"
                    }
                    option {
                        value: "driving",
                        selected: mode == Some(&TransportationMode::Driving),
                        "Driving"
                    }
                    option {
                        value: "walking",
                        selected: mode == Some(&TransportationMode::Walking),
                        "Walking"
                    }
                }
                input {
                    class: "time",
                    name: "time{k}",
                    key: "time{k}",
                    r#type: "number",
                    value: if let Some(time) = time { *time },
                    min: "1",
                    max: "999"
                }

            }
        }
    }
}

#[component]
pub fn CriteriaForm(
    criteria_colors: Signal<Vec<String>>,
    criteria_raw: Signal<Vec<(TransportationMode, String, usize, String)>>,
) -> Element {
    rsx! {
        form { id: "criteria_form",
               onsubmit: move |event| async move {
                   let values = event.values();
                   let mut criteria = vec![];
                   for (k, color) in criteria_colors().into_iter().enumerate() {
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
                           criteria.push((mode, address, time, color))
                       }
                   }

                   Ok(criteria_raw.set(criteria))
               },
               Criteria { criteria_colors, criteria_raw }
               input {
                   id: "add_criterion",
                   r#type: "button",
                   value: "Add",
                   onclick: move |_| criteria_colors.push(random_color::RandomColor::new().to_hex())
               }
               input {
                   id: "submit_search",
                   r#type: "submit",
                   value: "Ok"
               }
        }

    }
}
