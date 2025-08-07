use dioxus::prelude::*;

use crate::{Building, TransportationMode, components::ApartmentView};

#[component]
pub fn BuildingView(building: Building) -> Element {
    let mut times = building.times.iter().collect::<Vec<_>>();
    times.sort_by_key(|(k, _)| **k);
    rsx! {
        li { class: "building",
             div { class: "building-head",
                   h3 { "{building.name}" }
                   h4 {
                       for (_, (criterion, time)) in times {
                           span { class: "time-indicator",
                                  "style": "color: {criterion.color}",
                                  match criterion.mode {
                                      TransportationMode::Cycling =>
                                          rsx! { i { class: "fa-solid fa-person-biking" } },
                                      TransportationMode::Walking =>
                                          rsx! { i { class: "fa-solid fa-person-walking" } },
                                      TransportationMode::Driving =>
                                          rsx! { i { class: "fa-solid fa-car-side" } },
                                      TransportationMode::Public =>
                                          rsx! { i { class: "fa-solid fa-train-subway" } },
                                  }
                                  "{(*time as f32 / 60.0) as usize}"
                           }
                       }
                   }
             }
             ul { class: "apartments",
                  for apartment in building.apartments {
                      ApartmentView { name: building.name.clone(), apartment: apartment }
                  }
             }
        }
    }
}
