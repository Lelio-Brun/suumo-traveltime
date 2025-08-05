use dioxus::prelude::*;

use crate::{Building, components::ApartmentView};

#[component]
pub fn BuildingView(building: Building) -> Element {
    let times = building
        .times
        .iter()
        .map(|(k, time)| format!("{k}: {}", (*time as f32 / 60.0) as usize))
        .collect::<Vec<String>>();
    let times = &times[..].join(", ");
    rsx! {
        li { class: "building",
             div { class: "building-head",
                   h3 { "{building.name}" }
                   h4 {
                       i { class: "fa-solid fa-clock-rotate-left" }
                       {times.clone()}
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
