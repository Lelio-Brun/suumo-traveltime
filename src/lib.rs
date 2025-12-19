use std::{
    collections::HashMap,
    num::{ParseFloatError, ParseIntError},
};

use dioxus::html::{FormData, FormValue};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod backend;
pub mod components;
mod geocode;
mod scrape;

const SUUMOURL: &str = "https://suumo.jp/jj/chintai/ichiran/FR301FC001/?url=%2Fchintai%2Fichiran%2FFR301FC001%2F&ar=030&bs=040&pc=50&smk=&po1=25&po2=99&tc=0400501&tc=0400902&shkr1=03&shkr2=03&shkr3=03&shkr4=03&cb=0.0&ct=13.0&md=03&md=04&md=05&md=06&md=07&md=08&md=09&md=10&md=11&md=12&md=13&md=14&et=9999999&mb=25&mt=9999999&cn=9999999&ta=13&sc=13103&sc=13104&sc=13113&sc=13110&sc=13112";

#[derive(Error, Debug)]
pub enum Error {
    #[error("server error: {0}")]
    ServerError(#[from] dioxus::CapturedError),
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("serde_json error: {0}")]
    SerdeJSON(#[from] serde_json::Error),
    #[error("parse error: {0}")]
    ParseInt(#[from] ParseIntError),
    #[error("parse error: {0}")]
    ParseFloat(#[from] ParseFloatError),
    #[error("selector error: {0}")]
    Scrape(String),
    #[error("misc error: {0}")]
    Misc(String),
}

impl<'a> From<scraper::error::SelectorErrorKind<'a>> for Error {
    fn from(error: scraper::error::SelectorErrorKind<'a>) -> Self {
        Self::Scrape(error.to_string())
    }
}

impl From<dioxus::prelude::ServerFnError> for Error {
    fn from(error: dioxus::prelude::ServerFnError) -> Self {
        Self::ServerError(error.into())
    }
}

#[derive(Clone, PartialEq)]
pub struct Apartment {
    rent: String,
    fees: Option<String>,
    id: u64,
    deposit: Option<String>,
    key_money: Option<String>,
    kind: String,
    area: String,
    plan: String,
    url: String,
}

#[derive(Clone, PartialEq)]
pub struct Building {
    name: String,
    address: String,
    coordinates: (f64, f64),
    times: HashMap<usize, (Criterion, usize)>,
    apartments: Vec<Apartment>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
enum TransportationMode {
    Cycling,
    Driving,
    Walking,
    Public,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Criterion {
    mode: TransportationMode,
    address: String,
    time: usize,

    #[serde(default = "random_color")]
    color: String,

    #[serde(skip)]
    location: (f64, f64),
}

pub fn random_color() -> String {
    random_color::RandomColor::new().to_hex()
}

fn form_value_to_string(value: &FormValue) -> Option<String> {
    match value {
        FormValue::Text(s) => Some(s.clone()),
        _ => None,
    }
}

pub fn get_string(data: &FormData, field: &str) -> Option<String> {
    data.get_first(field)
        .as_ref()
        .and_then(form_value_to_string)
}
