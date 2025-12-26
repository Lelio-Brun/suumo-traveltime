use chrono::{Datelike, Days, Local, NaiveTime, Weekday};
use reqwest::{Client, RequestBuilder};

use crate::{Building, Criterion, Error, TransportationMode, backend};

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

// pub fn geocode_request(app_id: &str, api_key: &str) -> ClonableRequestBuilder {
//     let url = "https://api.traveltimeapp.com/v4/geocoding/search";
//     let geocode_client = Client::new();
//     let geocode_request = geocode_client
//         .get(url)
//         .header("X-Application-Id", app_id)
//         .header("X-Api-Key", api_key)
//         .header("Accept-Language", "en-US");
//     ClonableRequestBuilder(geocode_request)
// }

pub fn geocode_request(app_id: &str, api_key: &str) -> ClonableRequestBuilder {
    let url = "https://geocode.googleapis.com/v4beta/geocode/address";
    let geocode_client = Client::new();
    let geocode_request = geocode_client
        .get(url)
        .header("X-Goog-Api-Key", api_key)
        .header("X-Goog-FieldMask", "results.location");
    ClonableRequestBuilder(geocode_request)
}

pub async fn geocode(address: &str, request: RequestBuilder) -> Result<(f64, f64), Error> {
    match backend::get_coords(address.to_string()).await {
        Err(_) => {
            let text = request
                // .query(&[("query", address)])
                .query(&[("addressQuery", address)])
                .send()
                .await?
                .text()
                .await?;
            let json: serde_json::Value = serde_json::from_str(&text)?;

            // let coordinates = &json["features"][0]["geometry"]["coordinates"];
            // let lng = coordinates[0].as_f64().unwrap();
            // let lat = coordinates[1].as_f64().unwrap();
            let coordinates = &json["results"][0]["location"];
            let lng = coordinates["longitude"].as_f64().unwrap();
            let lat = coordinates["latitude"].as_f64().unwrap();

            backend::set_coords(address.to_string(), lng, lat).await?;

            Ok((lng, lat))
        }
        Ok(ok) => Ok(ok),
    }
}

fn next_monday() -> String {
    let now = Local::now();
    let morning = NaiveTime::from_hms_opt(8, 0, 0).unwrap();
    let t = now.with_time(morning).unwrap();
    let next = match t.weekday() {
        Weekday::Mon => t + Days::new(7),
        Weekday::Tue => t + Days::new(6),
        Weekday::Wed => t + Days::new(5),
        Weekday::Thu => t + Days::new(4),
        Weekday::Fri => t + Days::new(3),
        Weekday::Sat => t + Days::new(2),
        Weekday::Sun => t + Days::new(1),
    };
    next.to_rfc3339()
}

pub async fn get_travel_time(
    app_id: &str,
    api_key: &str,
    buildings: &mut [Building],
    criteria: &[Criterion],
) -> Result<(), Error> {
    let url = "https://routes.googleapis.com/distanceMatrix/v2:computeRouteMatrix";
    let client = Client::new();
    let request = client.post(url).header("X-Goog-Api-Key", api_key).header(
        "X-Goog-FieldMask",
        "originIndex,destinationIndex,duration,condition",
    );

    let set_time = |building: &mut Building, i: usize, criterion: &Criterion, time: usize| {
        if time <= criterion.time * 60 {
            building.times.insert(i, (criterion.clone(), time));
        }
    };

    for (i, criterion) in criteria.iter().enumerate() {
        let origin = serde_json::json!(
        {
            "waypoint": {
                "location": {
                    "latLng": {
                        "latitude": criterion.location.1,
                        "longitude": criterion.location.0
                    }
                }
            }
        });
        let (limit, mode) = match criterion.mode {
            TransportationMode::Cycling => (625, "BICYCLE"),
            TransportationMode::Walking => (625, "WALK"),
            TransportationMode::Driving => (625, "DRIVE"),
            TransportationMode::Public => (100, "TRANSIT"),
        };

        let mut new_buildings = vec![];
        for building in buildings.iter_mut() {
            match backend::get_time(
                criterion.address.clone(),
                building.address.clone(),
                criterion.mode.clone(),
            )
            .await
            {
                Ok(time) => set_time(building, i, criterion, time),
                Err(_) => new_buildings.push(building),
            }
        }

        for buildings_batch in new_buildings.chunks_mut(limit) {
            let destinations = buildings_batch
                .iter()
                .map(|building| {
                    serde_json::json!(
                    {
                        "waypoint": {
                            "location": {
                                "latLng": {
                                    "latitude": building.coordinates.1,
                                    "longitude": building.coordinates.0
                                }
                            }
                        }
                    })
                })
                .collect::<Vec<_>>();

            let body = serde_json::json!({
                "origins": [origin],
                "destinations": destinations,
                "travelMode": mode,
                "departureTime": next_monday(),
            });

            let json: serde_json::Value = request
                .try_clone()
                .unwrap()
                .json(&body)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;

            for route in json.as_array().unwrap() {
                if route["condition"].as_str().unwrap() == "ROUTE_EXISTS" {
                    let time = route["duration"]
                        .as_str()
                        .unwrap()
                        .strip_suffix("s")
                        .unwrap()
                        .parse::<f64>()
                        .unwrap() as usize;
                    let j = route["destinationIndex"].as_u64().unwrap() as usize;
                    let building = &mut buildings_batch[j];
                    backend::set_time(
                        criterion.address.clone(),
                        building.address.clone(),
                        criterion.mode.clone(),
                        time,
                    )
                    .await?;
                    set_time(building, i, criterion, time)
                }
            }
        }
    }

    Ok(())
}

// pub async fn get_travel_time(
//     app_id: &str,
//     api_key: &str,
//     buildings: &mut [Building],
//     criteria: &Vec<Criterion>,
// ) -> Result<(), Error> {
//     let n = criteria.len();
//     let mut table = HashMap::new();
//     for (k, building) in buildings.iter_mut().enumerate() {
//         table.insert(k + n, building);
//     }

//     let mut locations: Vec<serde_json::Value> = table
//         .iter()
//         .map(|(k, building)| {
//             let mut c = serde_json::Map::new();
//             c.insert("lng".to_owned(), building.coordinates.0.into());
//             c.insert("lat".to_owned(), building.coordinates.1.into());

//             let mut o = serde_json::Map::new();
//             o.insert("id".to_owned(), format!("{k}").into());
//             o.insert("coords".to_owned(), c.into());
//             o.into()
//         })
//         .collect();
//     for (k, criterion) in criteria.iter().enumerate() {
//         locations.push(serde_json::json!({"id": format!("{k}"),
//                                "coords": {"lng": criterion.location.0,
//                                           "lat": criterion.location.1}}));
//     }

//     let ids = (n..n + table.len())
//         .map(|k| format!("{k}"))
//         .collect::<Vec<String>>();

//     let searches = criteria
//         .iter()
//         .enumerate()
//         .map(|(k, criterion)| {
//             serde_json::json!(
//             {
//                 "id": format!("{k}"),
//                 "arrival_location_id": format!("{k}"),
//                 "departure_location_ids": ids,
//                 "travel_time": criterion.time * 60,
//                 "arrival_time_period": "weekday_morning",
//                 "transportation": {
//                     "type": match criterion.mode {
//                         TransportationMode::Cycling => "cycling+ferry",
//                         TransportationMode::Walking => "walking+ferry",
//                         TransportationMode::Driving => "driving+ferry",
//                         TransportationMode::Public => "public_transport",
//                     }
//                 },
//                 "properties": [
//                     "travel_time"
//                 ],
//             })
//         })
//         .collect::<Vec<serde_json::Value>>();

//     let body = serde_json::json!({
//         "locations": locations,
//         "arrival_searches": {
//             "many_to_one": searches
//         }
//     });

//     let url = "https://routes.googleapis.com/distanceMatrix/v2:computeRouteMatrix";
//     // let url = "https://api.traveltimeapp.com/v4/time-filter/fast";
//     let client = Client::new();
//     let json: serde_json::Value = client
//         .post(url)
//         .header("X-Goog-Api-Key", api_key)
//         .header(
//             "X-Goog-FieldMask",
//             "originIndex,destinationIndex,duration,condition",
//         )
//         // .header("X-Application-Id", app_id)
//         // .header("X-Api-Key", api_key)
//         // .header("Content-Type", "application/json")
//         // .header("Accept-Language", "en-US")
//         .json(&body)
//         .send()
//         .await?
//         .error_for_status()?
//         .json()
//         .await?;
//     // let json: serde_json::Value = serde_json::from_str(&text)?;

//     let json = &json["results"];
//     for j in 0..n {
//         let locations = json[j]["locations"].as_array().unwrap();

//         for location in locations {
//             let k: usize = location["id"].as_str().unwrap().parse()?;
//             let time: usize = location["properties"]["travel_time"].as_i64().unwrap() as usize;
//             table.entry(k).and_modify(|building| {
//                 let criterion = criteria.into_iter().nth(j).unwrap().clone();
//                 building.times.insert(j, (criterion, time));
//             });
//         }
//     }

//     Ok(())
// }
