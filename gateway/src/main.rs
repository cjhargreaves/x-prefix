mod proxy;
mod state;

use axum::{Router, routing::post};
use state::AppState;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let app = Router::new()
        .route("/v1/chat/completions", post(proxy::handle))
        .with_state(AppState::from_env());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
