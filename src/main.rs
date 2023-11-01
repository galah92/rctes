mod db;

use askama::Template;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let pool = PgPoolOptions::new()
        .connect("postgres://postgres:postgres@localhost:5432/postgres")
        .await?;

    let state = AppState { pool };

    let app = Router::new()
        .route("/", get(index))
        .route("/clicked", post(clicked))
        .route("/location", get(location))
        .route("/healthcheck", get(|| async { StatusCode::OK }))
        .with_state(state)
        .nest_service("/assets", ServeDir::new("assets"));

    let addr = "127.0.0.1:3000".parse().unwrap();
    println!("Listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

#[derive(Clone)]
struct AppState {
    pool: PgPool,
}

async fn index(
    Query(query): Query<IndexParams>,
    State(AppState { pool, .. }): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let name = query.name.unwrap_or_else(|| "World".to_string());
    let locations = db::get_all_locations(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Index { name, locations })
}

#[derive(Deserialize)]
struct IndexParams {
    name: Option<String>,
}

#[derive(Template)]
#[template(path = "index.html")]
struct Index {
    name: String,
    locations: Vec<db::Location>,
}

async fn clicked() -> impl IntoResponse {
    Clicked {}.into_response()
}

#[derive(Template)]
#[template(path = "clicked.html")]
struct Clicked {}

async fn location(
    Query(query): Query<LocationParams>,
    State(AppState { pool, .. }): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let name = query.name.ok_or(StatusCode::BAD_REQUEST)?;

    let location = db::get_location(&pool, &name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let population = location.ok_or(StatusCode::NOT_FOUND)?.population;

    let parents = db::get_parents(&pool, &name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let template = Location {
        name,
        population,
        parents,
    };
    Ok(template)
}

#[derive(Deserialize)]
struct LocationParams {
    name: Option<String>,
}

#[derive(Template)]
#[template(path = "location.html")]
struct Location {
    name: String,
    population: i64,
    parents: Vec<String>,
}
