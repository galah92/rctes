mod db;

use askama::Template;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::{Request, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Form, Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
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
            let request_id = request
                .headers()
                .get("x-request-id")
                .and_then(|value| value.to_str().ok())
                .map(|value| value.to_string())
                .unwrap_or_else(|| Uuid::new_v4().to_string());

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
        .route("/counter", get(counter))
        .route("/clicked", post(clicked))
        .route("/location", post(create_location))
        .route("/location", get(location))
        .route("/healthcheck", get(|| async { StatusCode::OK }))
        .with_state(state)
        .nest_service("/assets", ServeDir::new("assets"))
        .layer(trace_layer);

    let addr = "127.0.0.1:3000";
    tracing::info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[derive(Clone)]
struct AppState {
    pool: PgPool,
}

impl From<db::DbError> for StatusCode {
    fn from(error: db::DbError) -> Self {
        match error {
            db::DbError::NotFound => StatusCode::NOT_FOUND,
            db::DbError::Other(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
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
    db::create_location(&pool, &location).await?;
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
    let name = query.name.unwrap_or_else(|| "World".to_string());
    let locations = db::get_all_locations(&pool).await?;
    Ok(HtmlTemplate(Index { name, locations }))
}

struct HtmlTemplate<T>(T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(error) => {
                tracing::error!("Failed to render template: {}", error);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
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

async fn counter(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(|socket: WebSocket| async move {
        let (mut tx, _) = socket.split();

        let mut count = 0usize;
        loop {
            let counter = Counter { count };
            let message = Message::Text(counter.render().unwrap());
            if let Err(error) = tx.send(message).await {
                tracing::warn!("Failed to send message: {}", error);
                break;
            }
            count += 1;
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    })
}

#[derive(Template)]
#[template(path = "counter.html")]
struct Counter {
    count: usize,
}

async fn clicked() -> impl IntoResponse {
    HtmlTemplate(Clicked {})
}

#[derive(Template)]
#[template(path = "clicked.html")]
struct Clicked {}

async fn location(
    Query(query): Query<LocationParams>,
    State(AppState { pool, .. }): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let name = query.name.ok_or(StatusCode::BAD_REQUEST)?;
    let location = db::get_location(&pool, &name).await?;
    let population = location.ok_or(StatusCode::NOT_FOUND)?.population;
    let parents = db::get_parents(&pool, &name).await?;

    let template = Location {
        name,
        population,
        parents,
    };
    Ok(HtmlTemplate(template))
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
