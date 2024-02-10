use askama::Template;
use axum::extract::State;
use axum_extra::extract::Form;

use crate::{
    anita::{Anita, AnitaForm},
    db, factuur,
    server::AppState,
    Page,
};

use super::factuur::FactuurTemplate;

#[derive(Template)]
#[template(path = "anita.html")]
pub struct AnitaTemplate {
    page: Page,
}

pub async fn get() -> AnitaTemplate {
    AnitaTemplate { page: Page::Anita }
}

pub async fn post(
    State(state): State<AppState>,
    Form(anita_form): Form<AnitaForm>,
) -> FactuurTemplate {
    let (year, month) = anita_form.maand.0.split_once('-').unwrap();
    let items = match Anita::new("Noemi".to_string())
        .get_events_from_month(month.to_owned(), year.to_owned())
        .await
    {
        Ok(events) => events
            .iter()
            .cloned()
            .filter_map(|e| factuur::WorkItem::try_from(e).ok())
            .collect::<Vec<factuur::WorkItem>>(),
        Err(err) => {
            println!("Failed to fetch data from L1NDA: {}", err);
            vec![]
        }
    };

    let mut conn = state.db.acquire().await.unwrap();

    let anita = match db::get_client(&mut conn, "V.O.F. De Nieuwe Anita").await {
        Ok(a) => a,
        Err(_) => None,
    };

    let most_recent_invoice_id = match db::most_recent_invoice(&mut conn).await {
        Ok(i) => i,
        Err(_) => None,
    };

    FactuurTemplate {
        page: Page::Factuur,
        client: anita,
        items,
        most_recent_invoice_id,
    }
}
