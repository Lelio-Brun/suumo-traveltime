use dioxus::prelude::*;

use crate::Criterion;

#[cfg(feature = "server")]
use crate::{ADDRESS, DESTCOLOR, TIMEOUT, TransportationMode};

#[cfg(feature = "server")]
thread_local! {
    pub static DB: rusqlite::Connection = {
        let conn = rusqlite::Connection::open("data.db").expect("Failed to open database");

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS buildings (
                address TEXT PRIMARY KEY,
                lat REAL,
                lng REAL,
                reachable INTEGER,
                time INTEGER
            );
            CREATE TABLE IF NOT EXISTS credentials (
                app_id TEXT NOT NULL,
                key TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS criteria (
                address TEXT NOT NULL,
                mode TEXT NOT NULL,
                time INTEGER,
                color TEXT NOT NULL);").unwrap();

        conn
    };
}

#[server]
pub async fn save_credentials(app_id: String, api_key: String) -> Result<(), ServerFnError> {
    DB.with(|db| db.execute("DELETE FROM credentials", []))?;
    DB.with(|db| db.execute("INSERT INTO credentials VALUES (?1, ?2)", (app_id, api_key)))?;
    Ok(())
}

#[server]
pub async fn get_credentials() -> Result<(String, String), ServerFnError> {
    Ok(DB.with(|db| {
        db.query_row("SELECT * FROM credentials", [], |row| {
            let app_id: String = row.get(0)?;
            let api_key: String = row.get(1)?;
            Ok((app_id, api_key))
        })
    })?)
}

#[server]
pub async fn get_coords(address: String) -> Result<(f64, f64), ServerFnError> {
    Ok(DB.with(|db| {
        db.query_row(
            "SELECT lat, lng FROM buildings WHERE address = ?1",
            [address],
            |row| {
                let lat: f64 = row.get(0)?;
                let lng: f64 = row.get(1)?;
                Ok((lng, lat))
            },
        )
    })?)
}

#[server]
pub async fn set_coords(address: String, lng: f64, lat: f64) -> Result<(), ServerFnError> {
    DB.with(|db| {
        db.execute(
            "INSERT INTO buildings VALUES (?1, ?2, ?3, NULL, NULL) ON CONFLICT DO NOTHING",
            (address, lat, lng),
        )
    })?;
    Ok(())
}

#[server]
pub async fn get_criteria() -> Result<Vec<Criterion>, ServerFnError> {
    let mut criteria: Vec<Criterion> = DB.with(|db| {
        let mut query = db.prepare("SELECT * FROM criteria")?;
        let criteria = query
            .query_map([], move |row| {
                let address: String = row.get(0)?;
                let mode: String = row.get(1)?;
                let mode: TransportationMode =
                    serde_json::from_str(&format!(r#""{}""#, mode)).unwrap();
                let time: usize = row.get(2)?;
                let color: String = row.get(3)?;
                let location = (0.0, 0.0);
                Ok(Criterion {
                    address,
                    mode,
                    time,
                    color,
                    location,
                })
            })?
            .collect::<Result<Vec<Criterion>, _>>();
        criteria
    })?;

    if criteria.is_empty() {
        criteria = vec![Criterion {
            mode: TransportationMode::Cycling,
            address: ADDRESS.to_string(),
            time: TIMEOUT,
            color: DESTCOLOR.to_string(),
            location: (0.0, 0.0),
        }]
    }

    Ok(criteria)
}

#[server]
pub async fn set_criteria(criteria: Vec<Criterion>) -> Result<(), ServerFnError> {
    DB.with(|db| {
        db.execute_batch(&format!(
            "DELETE FROM criteria;
             INSERT INTO criteria VALUES {};",
            criteria
                .iter()
                .map(|criterion| format!(
                    r#"("{}", {}, {}, "{}")"#,
                    criterion.address,
                    serde_json::to_string(&criterion.mode).unwrap(),
                    criterion.time,
                    criterion.color
                ))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    })?;
    Ok(())
}
