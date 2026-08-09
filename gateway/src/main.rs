mod allocator;
mod prefix;
mod proxy;
mod state;

use axum::{
    Router,
    routing::{get, post},
};
use state::AppState;

#[tokio::main]
async fn main() {
    dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/.env")).ok();

    let state = AppState::from_env();
    state.allocator.refresh(&state.client).await;
    println!("prefix allocator: {} trends loaded", state.allocator.active_count());

    spawn_refresh_loop(state.clone());

    let app = Router::new()
        .route("/v1/chat/completions", post(proxy::handle))
        .route("/trends", get(proxy::list_trends))
        .route("/trends/{key}", get(proxy::get_trend))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn spawn_refresh_loop(state: AppState) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(state.refresh_interval).await;
            state.allocator.refresh(&state.client).await;
            println!(
                "prefix allocator: refreshed, {} trends",
                state.allocator.active_count()
            );
        }
    });
}
