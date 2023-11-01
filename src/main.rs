mod db;

use askama::Template;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
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
) -> impl IntoResponse {
    let name = query.name.unwrap_or_else(|| "World".to_string());
    let locations = db::get_all_locations(&pool).await.unwrap();
    Index { name, locations }.into_response()
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

async fn location(
    Query(query): Query<LocationParams>,
    State(AppState { pool, .. }): State<AppState>,
) -> impl IntoResponse {
    let Some(name) = query.name else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let location = db::get_location(&pool, &name).await.unwrap();
    let Some(location) = location else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let population = location.population;

    let parents = db::get_parents(&pool, &name).await.unwrap();

    let template = Location {
        name,
        population,
        parents,
    };
    template.into_response()
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
