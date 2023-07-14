use anyhow::Result;
use sqlx::SqliteConnection;

use crate::factuur::Factuur;

pub async fn add_invoice_to_db(conn: &mut SqliteConnection, factuur: &Factuur) -> Result<()> {
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

    println!("Client ID: {}", client.id);

    // Calculate some additional information to store alongside the mvp
    // invoice in the database.
    let nummer = factuur.nummer as i32;
    let work_items = serde_json::to_string(&factuur.work_items)?;
    let subtotal = factuur.work_items.iter().map(|i| i.euro).sum::<f32>();
    let btw = subtotal * 0.21;
    let total = subtotal + btw;

    // Insert the new invoice into the database
    sqlx::query!(
        r#"
INSERT INTO invoice ( nummer, client, work_items, subtotal, btw, total )
VALUES ( ?, ?, ?, ?, ?, ? )
        "#,
        nummer,
        client.id,
        work_items,
        subtotal,
        btw,
        total
    )
    .execute(&mut *conn)
    .await?;

    Ok(())
}
