use anyhow::{anyhow, Result};
use chrono::{TimeZone, Utc};
use sqlx::SqliteConnection;

use crate::factuur::{Client, Factuur};

pub async fn add_invoice(conn: &mut SqliteConnection, factuur: &Factuur) -> Result<()> {
    // First make sure that the respective Client entry exists
    sqlx::query!(
        r#"
INSERT OR IGNORE INTO client ( name, address, zip )
VALUES ( ?, ?, ? )
        "#,
        factuur.client.name,
        factuur.client.address,
        factuur.client.zip
    )
    .execute(&mut *conn)
    .await?;

    let client = sqlx::query!(
        r#"
SELECT id FROM client WHERE name = ?
        "#,
        factuur.client.name
    )
    .fetch_one(&mut *conn)
    .await?;

    // Calculate some additional information to store alongside the mvp
    // invoice in the database.
    let nummer = factuur.nummer as i32;
    let work_items = serde_json::to_string(&factuur.work_items)?;

    // Insert the new invoice into the database
    sqlx::query!(
        r#"
INSERT INTO invoice ( nummer, client, work_items, subtotal, btw, total, created_at )
VALUES ( ?, ?, ?, ?, ?, ?, ? )
        "#,
        nummer,
        client.id,
        work_items,
        factuur.subtotal,
        factuur.btw,
        factuur.total,
        factuur.date
    )
    .execute(&mut *conn)
    .await?;

    Ok(())
}

pub async fn get_invoices(conn: &mut SqliteConnection) -> Result<Vec<Factuur>> {
    let invoices = sqlx::query!(
        r#"
SELECT nummer, client.name, client.address, client.zip, work_items, subtotal, btw, total, created_at FROM invoice
INNER JOIN client ON client.id = invoice.client
        "#
    )
    .fetch_all(&mut *conn)
    .await?;

    let res = invoices
        .iter()
        .map(|row| Factuur {
            nummer: row.nummer as usize,
            client: Client {
                name: row.name.clone(),
                address: row.address.clone(),
                zip: row.zip.clone(),
            },
            work_items: serde_json::from_str(&row.work_items).unwrap(),
            subtotal: row.subtotal,
            btw: row.btw,
            total: row.total,
            date: Utc.from_local_datetime(&row.created_at).unwrap(),
        })
        .collect();

    Ok(res)
}

pub async fn get_all_clients(conn: &mut SqliteConnection) -> Result<Vec<Client>> {
    sqlx::query_as!(Client, "SELECT name, address, zip FROM client")
        .fetch_all(&mut *conn)
        .await
        .map_err(|err| anyhow!(err))
}

pub async fn get_client(conn: &mut SqliteConnection, client_name: &str) -> Result<Option<Client>> {
    sqlx::query_as!(
        Client,
        r#"
SELECT name, address, zip FROM client
WHERE name = ?
        "#,
        client_name
    )
    .fetch_optional(&mut *conn)
    .await
    .map_err(|err| anyhow!(err))
}

pub async fn most_recent_invoice(conn: &mut SqliteConnection) -> Result<Option<usize>> {
    let res = sqlx::query!("SELECT MAX(nummer) AS nummer FROM invoice")
        .fetch_one(&mut *conn)
        .await?;

    match res.nummer {
        Some(n) => Ok(Some(n as usize)),
        None => Ok(None),
    }
}
