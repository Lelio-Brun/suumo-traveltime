use std::collections::HashMap;

use reqwest::{Client, RequestBuilder};
use serde_json;

use crate::{Building, Criterion, Error, TransportationMode, backend};

pub async fn geocode<'a>(address: &'a str, request: RequestBuilder) -> Result<(f64, f64), Error> {
    match backend::get_coords(address.to_string()).await {
        Err(_) => {
            let text = request
                .query(&[("query", address)])
                .send()
                .await?
                .text()
                .await?;
            let json: serde_json::Value = serde_json::from_str(&text)?;

            let coordinates = &json["features"][0]["geometry"]["coordinates"];
            let lng = coordinates[0].as_f64().unwrap();
            let lat = coordinates[1].as_f64().unwrap();

            backend::set_coords(address.to_string(), lng, lat).await?;

            Ok((lng, lat))
        }
        Ok(ok) => Ok(ok),
    }
}

pub async fn get_travel_time<'a>(
    app_id: &'a str,
    api_key: &'a str,
    buildings: &mut Vec<Building>,
    criteria: &Vec<Criterion>,
) -> Result<(), Error> {
    let n = criteria.len();
    let mut table = HashMap::new();
    for (k, building) in buildings.iter_mut().enumerate() {
        table.insert(k + n, building);
    }

    let mut locations: Vec<serde_json::Value> = table
        .iter()
        .map(|(k, building)| {
            let mut c = serde_json::Map::new();
            c.insert("lng".to_owned(), building.coordinates.0.clone().into());
            c.insert("lat".to_owned(), building.coordinates.1.clone().into());

            let mut o = serde_json::Map::new();
            o.insert("id".to_owned(), format!("{k}").into());
            o.insert("coords".to_owned(), c.into());
            o.into()
        })
        .collect();
    for (k, criterion) in criteria.iter().enumerate() {
        locations.push(serde_json::json!({"id": format!("{k}"),
                               "coords": {"lng": criterion.location.0,
                                          "lat": criterion.location.1}}));
    }

    let ids = (n..n + table.len())
        .map(|k| format!("{k}"))
        .collect::<Vec<String>>();

    let searches = criteria
        .iter()
        .enumerate()
        .map(|(k, criterion)| {
            serde_json::json!(
            {
                "id": format!("{k}"),
                "arrival_location_id": format!("{k}"),
                "departure_location_ids": ids,
                "travel_time": criterion.time * 60,
                "arrival_time_period": "weekday_morning",
                "transportation": {
                    "type": match criterion.mode {
                        TransportationMode::Cycling => "cycling+ferry",
                        TransportationMode::Walking => "walking+ferry",
                        TransportationMode::Driving => "driving+ferry",
                        TransportationMode::Public => "public_transport",
                    }
                },
                "properties": [
                    "travel_time"
                ],
            })
        })
        .collect::<Vec<serde_json::Value>>();

    let body = serde_json::json!({
        "locations": locations,
        "arrival_searches": {
            "many_to_one": searches
        }
    });

    let url = "https://api.traveltimeapp.com/v4/time-filter/fast";
    let client = Client::new();
    let text = client
        .post(url)
        .header("X-Application-Id", app_id)
        .header("X-Api-Key", api_key)
        .header("Content-Type", "application/json")
        .header("Accept-Language", "en-US")
        .json(&body)
        .send()
        .await?
        .text()
        .await?;
    let json: serde_json::Value = serde_json::from_str(&text)?;

    let json = &json["results"];
    for j in 0..n {
        let locations = json[j]["locations"].as_array().unwrap();

        for location in locations {
            let k: usize = location["id"].as_str().unwrap().parse()?;
            let time: usize = location["properties"]["travel_time"].as_i64().unwrap() as usize;
            table.entry(k).and_modify(|building| {
                let criterion = criteria.into_iter().nth(j).unwrap().clone();
                building.times.insert(j, (criterion, time));
            });
        }
    }

    Ok(())
}
