use dioxus::prelude::*;

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
            );").unwrap();

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
