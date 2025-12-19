use std::collections::HashMap;

use dioxus::prelude::*;
use dioxus_logger::tracing;
use reqwest::{Client, RequestBuilder};
use scraper::{Html, Selector};

use crate::{Apartment, Building, Error, backend, geocode::geocode};

pub async fn scrape(
    mut scrape_progress: Signal<f64>,
    geocode_request: RequestBuilder,
) -> Result<Vec<Building>, Error> {
    let url = backend::get_suumo_url().await?;
    tracing::debug!("scraping {url}");
    let url = format!("https://corsproxy.io/?url={url}");
    // let url = format!("https://crossorigin.me/{url}");

    let client = Client::new();
    let request = client.get(url);

    let html = request.try_clone().unwrap().send().await?.text().await?;
    // let html = include_str!("../suumo.html");

    let mut doc = Html::parse_document(&html);

    let total_sel = Selector::parse("div.paginate_set-hit")?;
    let mut total = doc
        .select(&total_sel)
        .next()
        .ok_or(Error::Scrape("count not found".to_string()))?
        .text()
        .next()
        .ok_or(Error::Scrape("number not found".to_string()))?
        .trim()
        .to_string();
    total.retain(|c| c != ',');
    let total: f64 = total.parse()?;

    let pagination_sel = Selector::parse("ol.pagination-parts")?;
    let pages: usize = doc
        .select(&pagination_sel)
        .next()
        .ok_or(Error::Scrape("pagination not found".to_string()))?
        .child_elements()
        .last()
        .ok_or(Error::Scrape("last page not found".to_string()))?
        .text()
        .collect::<String>()
        .parse()?;

    let building_sel = Selector::parse("div.cassetteitem")?;
    let name_sel = Selector::parse("div.cassetteitem_content-title")?;
    let address_sel = Selector::parse("li.cassetteitem_detail-col1")?;
    let apartment_sel = Selector::parse("tr.js-cassette_link")?;
    let rent_sel = Selector::parse("span.cassetteitem_price--rent")?;
    let fees_sel = Selector::parse("span.cassetteitem_price--administration")?;
    let deposit_sel = Selector::parse("span.cassetteitem_price--deposit")?;
    let key_money_sel = Selector::parse("span.cassetteitem_price--gratuity")?;
    let kind_sel = Selector::parse("span.cassetteitem_madori")?;
    let area_sel = Selector::parse("span.cassetteitem_menseki")?;
    let plan_sel = Selector::parse("img.casssetteitem_other-thumbnail-img")?;
    let url_sel = Selector::parse("a.cassetteitem_other-linktext")?;
    let id_sel = Selector::parse("input#bukken_0")?;

    let mut buildings = vec![];
    let mut apt_count = 0.0;

    for page in 1..=1 {
        tracing::debug!("Page {page} / {pages}");

        if page > 1 {
            let html = request
                .try_clone()
                .unwrap()
                .query(&[("page", format!("{page}"))])
                .send()
                .await?
                .text()
                .await?;
            doc = Html::parse_document(&html);
        }

        for building in doc.select(&building_sel) {
            let find = |sel, sel_name| {
                Ok::<String, Error>(
                    building
                        .select(sel)
                        .next()
                        .ok_or(Error::Scrape(format!("{sel_name} not found")))?
                        .text()
                        .collect(),
                )
            };
            let name: String = find(&name_sel, "title")?;
            let address = find(&address_sel, "address")?;

            let request = geocode_request.try_clone().unwrap();
            let coordinates = geocode(&address, request).await?;

            let mut apartments = vec![];

            for apartment in building.select(&apartment_sel) {
                let find = |sel, sel_name| {
                    Ok::<String, Error>(
                        apartment
                            .select(sel)
                            .next()
                            .ok_or(Error::Scrape(format!("{name}: {sel_name} not found")))?
                            .text()
                            .collect(),
                    )
                };
                let rent = find(&rent_sel, "rent")?;
                let fees = find(&fees_sel, "fees")?;
                let fees = if fees == "-" { None } else { Some(fees) };
                let deposit = find(&deposit_sel, "deposit")?;
                let deposit = if deposit == "-" { None } else { Some(deposit) };
                let key_money = find(&key_money_sel, "key_money")?;
                let key_money = if key_money == "-" {
                    None
                } else {
                    Some(key_money)
                };
                let kind = find(&kind_sel, "kind")?;
                let area = find(&area_sel, "area")?;
                let plan = apartment
                    .select(&plan_sel)
                    .next()
                    .ok_or(Error::Scrape(format!("{name}: plan not found")))?
                    .attr("rel")
                    .ok_or(Error::Scrape(format!("{name}: rel not found")))?
                    .to_string();
                let url = apartment
                    .select(&url_sel)
                    .next()
                    .ok_or(Error::Scrape(format!("{name}: url not found")))?
                    .attr("href")
                    .ok_or(Error::Scrape(format!("{name}: href not found")))?
                    .to_string();
                let url = format!("https://suumo.jp{url}");
                let id = apartment
                    .select(&id_sel)
                    .next()
                    .ok_or(Error::Scrape(format!("{name}: id not found")))?
                    .attr("value")
                    .ok_or(Error::Scrape(format!("{name}: value not found")))?
                    .trim()
                    .parse::<u64>()?;

                apartments.push(Apartment {
                    rent,
                    fees,
                    deposit,
                    key_money,
                    kind,
                    area,
                    plan,
                    url,
                    id,
                });

                apt_count += 1.0;
                scrape_progress.set(apt_count / total);
            }

            buildings.push(Building {
                name,
                address,
                coordinates,
                apartments,
                times: HashMap::new(),
            });
        }
    }

    Ok(buildings)
}
