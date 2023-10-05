use argon2::{PasswordHash, PasswordVerifier};
use askama::Template;
use askama_axum::IntoResponse;
use axum::{extract::State, response::Redirect, Form};
use axum_login::{extractors::AuthContext, memory_store::MemoryStore as AuthMemoryStore};
use serde::Deserialize;

use crate::server::{AppState, User};

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {}

pub async fn login_get() -> LoginTemplate {
    LoginTemplate {}
}

#[derive(Clone, Debug, Deserialize)]
pub struct LoginForm {
    password: String,
}

pub async fn login_post(
    State(state): State<AppState>,
    mut auth: AuthContext<usize, User, AuthMemoryStore<usize, User>>,
    Form(login_form): Form<LoginForm>,
) -> axum::response::Result<impl IntoResponse> {
    let parsed_hash = match PasswordHash::new(&state.user.password_hash) {
        Ok(hash) => hash,
        Err(err) => return Err(format!("Failed to parse user's password hash: {err:#?}").into()),
    };

    match argon2::Argon2::default().verify_password(login_form.password.as_bytes(), &parsed_hash) {
        Ok(_) => match auth.login(&state.user).await {
            Ok(_) => Ok(Redirect::to("/")),
            Err(err) => Err(format!("Failed to login: {err:#?}").into()),
        },
        Err(err) => {
            println!("Authentication error: {err:#?}");
            Err(format!("Authentication failure: {err:#?}").into())
        }
    }
}
