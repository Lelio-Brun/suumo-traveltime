use dioxus::prelude::*;

use crate::Apartment;

#[component]
pub fn ApartmentView(name: String, apartment: Apartment) -> Element {
    rsx!(
        li { class: "apartment",
             onmouseenter:
                 move |_| {
                     let name = name.clone();
                     async move {
                         let _ = document::eval(&format!(r#"focusMarker("{name}");"#)).await;
                     }
                 },
             onmouseleave:  move |_| {
                 async move {
                     let _ = document::eval(r#"unfocusMarker();"#).await;
                 }
             },
             img {
                 src: "{apartment.plan}"
             }
             a {
                 href: "{apartment.url}",
                 target: "_blank",
                 div {
                     div { class: "layout",
                           span { class: "area",
                                  "{apartment.area}"
                           }
                           span { class: "kind",
                                  "{apartment.kind}"
                           }
                     }
                     div { class: "rent-fees",
                           span { class: "rent",
                                  i { class: "fa-solid fa-house" }
                                  "{apartment.rent}"
                           }
                           if apartment.fees.is_some() {
                               span { class: "fees",
                                      i { class: "fa-solid fa-list-check" }
                                      "{apartment.fees.as_deref().unwrap()}"
                               }
                           }
                     }
                     div { class: "initial-fees",
                           if apartment.deposit.is_some() {
                           span { class: "deposit",
                                  i { class: "fa-solid fa-money-bill-transfer" }
                                  "{apartment.deposit.as_deref().unwrap()}"
                           }
                           }
                           if apartment.key_money.is_some() {
                           span { class: "key_money",
                                  i { class: "fa-solid fa-key" }
                                  "{apartment.key_money.as_deref().unwrap()}"
                           }
                           }
                     }
                 }
             }
        }
    )
}
