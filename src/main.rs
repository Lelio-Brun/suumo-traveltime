use dioxus::html::events::*;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use suumo_traveltime::{backend, components};

const MAIN_CSS: Asset = asset!("/assets/main.css");
const MAP_JS: Asset = asset!("/assets/map.js", JsAssetOptions::new().with_minify(false));

fn main() {
    dioxus::launch(App);
}

#[derive(Deserialize, Serialize)]
struct Credentials {
    app_id: String,
    api_key: String,
}

#[component]
fn App() -> Element {
    let mut credentials = use_server_future(backend::get_credentials)?;

    let submit = move |event: FormEvent| async move {
        event.prevent_default();
        let Credentials { app_id, api_key } = event.parsed_values()?;
        backend::save_credentials(app_id, api_key).await?;
        credentials.restart();
        Ok(())
    };
    rsx! {
        document::Stylesheet {
            href: MAIN_CSS
        }
        document::Link {
            rel: "stylesheet",
            href: "https://unpkg.com/leaflet@1.9.4/dist/leaflet.css",
            integrity: "sha256-p4NxAoJBhIIN+hmNHrzRCf9tD/miZyoHS5obTRR9BMY=",
            crossorigin: ""
        }
        document::Script {
            src: "https://kit.fontawesome.com/546761b7ee.js",
            crossorigin: "anonymous"
        }
        document::Script {
            src: "https://unpkg.com/leaflet@1.9.4/dist/leaflet.js",
            integrity: "sha256-20nQCchB9co0qIjJZRGuk2/Z9VM+kNiyxNV1lvTlZBo=",
            crossorigin: ""
        }
        document::Script {
            src: MAP_JS
        }

        match credentials() {
            None => rsx! { div { "Checking database..." } },
            Some(Ok((app_id, api_key))) => rsx! {
                components::List { app_id: app_id, api_key: api_key }
            },
            Some(Err(e)) => {
                println!("{e}");
                rsx! {
                    form {
                        onsubmit: submit,
                        input { name: "app_id", placeholder: "App ID" }
                        input { name: "api_key", placeholder: "API Key" }
                        button { "Ok" }
                    }
                }
            }
        }

    }
}
