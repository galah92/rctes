mod db;

use askama::Template;
use axum::{
    extract::{Query, State},
    http::{Request, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Form, Router,
};
use serde::Deserialize;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tower_http::{
    services::ServeDir,
    trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    tracing_subscriber::fmt()
        // .json()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or("postgres://postgres:postgres@localhost:5432/postgres".into());
    let pool = PgPoolOptions::new().connect(&db_url).await?;

    let state = AppState { pool };

    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(|request: &Request<_>| {
            let request_id = Uuid::new_v4();
            tracing::info_span!(
                "request",
                method = %request.method(),
                path = %request.uri().path(),
                uri = %request.uri(),
                version = ?request.version(),
                request_id = %request_id,
            )
        })
        .on_request(DefaultOnRequest::default().level(tracing::Level::INFO))
        .on_response(DefaultOnResponse::default().level(tracing::Level::INFO));

    let app = Router::new()
        .route("/", get(index))
        .route("/clicked", post(clicked))
        .route("/location", post(create_location))
        .route("/location", get(location))
        .route("/healthcheck", get(|| async { StatusCode::OK }))
        .with_state(state)
        .nest_service("/assets", ServeDir::new("assets"))
        .layer(trace_layer);

    let addr = "127.0.0.1:3000".parse().unwrap();
    tracing::info!("Listening on {}", addr);

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

async fn create_location(
    State(AppState { pool, .. }): State<AppState>,
    Form(query): Form<CreateLocationParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let location = db::Location {
        name: query.name.ok_or(StatusCode::BAD_REQUEST)?,
        population: query.population.ok_or(StatusCode::BAD_REQUEST)?,
        parent: query.parent,
    };

    db::create_location(&pool, &location)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::CREATED)
}

#[derive(Deserialize)]
struct CreateLocationParams {
    name: Option<String>,
    population: Option<i64>,
    parent: Option<String>,
}

async fn index(
    Query(query): Query<IndexParams>,
    State(AppState { pool, .. }): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    tracing::info!("WIP");
    let name = query.name.unwrap_or_else(|| "World".to_string());
    let locations = db::get_all_locations(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Index { name, locations })
}

#[derive(Deserialize, Debug)]
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
