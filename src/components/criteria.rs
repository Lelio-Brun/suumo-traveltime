use dioxus::prelude::*;

use crate::{Criterion, Error, TransportationMode, backend};

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
                    // placeholder: if address.is_none() { "Address" },
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
    // criteria_colors: Signal<Vec<String>>,
    criteria_raw: Signal<Vec<Criterion>>,
) -> Element {
    rsx! {
        form { id: "criteria_form",
               onsubmit: move |event| async move {
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

                   Ok(criteria_raw.set(criteria))
               },
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
