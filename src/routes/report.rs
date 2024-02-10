use std::collections::HashMap;

use askama::Template;
use axum::extract::State;
use chrono::Datelike;

use crate::{
    db,
    factuur::Factuur,
    server::{filters, AppState},
    Page,
};

#[derive(Template)]
#[template(path = "history.html")]
pub struct HistoryTemplate {
    page: Page,
    grouped_invoices: Vec<(YearMonth, Vec<Factuur>)>,
}

#[derive(Template)]
#[template(path = "btw.html")]
pub struct BtwTemplate {
    page: Page,
    quarters: Vec<(Quarter, Btw)>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct YearMonth {
    year: i32,
    month: u32,
}

impl std::fmt::Display for YearMonth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let months = HashMap::from([
            (1, "Januari"),
            (2, "Februari"),
            (3, "Maart"),
            (4, "April"),
            (5, "Mei"),
            (6, "Juni"),
            (7, "Juli"),
            (8, "Augustus"),
            (9, "September"),
            (10, "Oktober"),
            (11, "November"),
            (12, "December"),
        ]);

        let display_month = match months.get(&self.month) {
            Some(m) => m,
            None => "Ooit",
        };

        write!(f, "{} {}", display_month.to_lowercase(), self.year)
    }
}

pub async fn history_get(State(state): State<AppState>) -> HistoryTemplate {
    let mut conn = state.db.acquire().await.unwrap();
    let mut invoices = match db::get_invoices(&mut conn).await {
        Ok(invoices) => invoices,
        Err(err) => {
            println!("Failed to fetch invoices from DB: {:?}", err);
            vec![]
        }
    };

    invoices.sort_by_key(|i| i.nummer);
    invoices.reverse();

    let mut grouped_invoices = HashMap::new();

    // Group by month
    for i in invoices {
        let year_month = YearMonth {
            year: i.date.year(),
            month: i.date.month(),
        };
        grouped_invoices.entry(year_month).or_insert(vec![]).push(i);
    }

    let mut grouped_invoices: Vec<(YearMonth, Vec<Factuur>)> = grouped_invoices
        .into_iter()
        .map(|(ym, invoices)| (ym, invoices))
        .collect();

    grouped_invoices.sort_by(|(ym1, _), (ym2, _)| ym1.cmp(ym2));
    grouped_invoices.reverse();

    HistoryTemplate {
        page: Page::Facturen,
        grouped_invoices,
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Quarter {
    year: i32,
    quarter: u32,
}

impl std::fmt::Display for Quarter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let quarters = HashMap::from([
            (1, "jan–maa"),
            (2, "apr–jun"),
            (3, "jul–sep"),
            (4, "okt–dec"),
        ]);

        let display_quarter = match quarters.get(&self.quarter) {
            Some(q) => q,
            None => "ooit",
        };

        write!(f, "{}Q{} ({})", self.year, self.quarter, display_quarter)
    }
}

#[derive(Debug, Clone)]
pub struct Btw {
    omzet: f64,
    btw: f64,
    invoices: Vec<Factuur>,
}

pub async fn btw_get(State(state): State<AppState>) -> BtwTemplate {
    let mut conn = state.db.acquire().await.unwrap();
    let mut invoices = match db::get_invoices(&mut conn).await {
        Ok(invoices) => invoices,
        Err(err) => {
            println!("Failed to fetch invoices from DB: {:?}", err);
            vec![]
        }
    };

    invoices.sort_by_key(|i| i.nummer);
    invoices.reverse();

    let mut grouped_invoices = HashMap::new();

    // Group by quarter
    for i in invoices {
        let q = Quarter {
            year: i.date.year(),
            // Default division behavior on unsigned integers is to floor
            quarter: ((i.date.month() as f32) / 3.0).ceil() as u32,
        };
        grouped_invoices.entry(q).or_insert(vec![]).push(i);
    }

    let mut grouped_invoices: Vec<(Quarter, Btw)> = grouped_invoices
        .into_iter()
        .map(|(q, invoices)| {
            let omzet = invoices.iter().map(|i| i.subtotal).sum();
            let btw = invoices.iter().map(|i| i.btw).sum();
            (
                q,
                Btw {
                    omzet,
                    btw,
                    invoices,
                },
            )
        })
        .collect();

    grouped_invoices.sort_by(|(ym1, _), (ym2, _)| ym1.cmp(ym2));
    grouped_invoices.reverse();

    BtwTemplate {
        page: Page::Btw,
        quarters: grouped_invoices,
    }
}
