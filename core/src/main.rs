use std::net::SocketAddr;

mod auth;
mod http;
mod note;
mod session;
mod user;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let app_state = http::state::AppState::new();
    let app = http::router::create_http_router(app_state);
    let addr = "0.0.0.0:40000";

    log::info!("Starting core server on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind core server listener");

    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .expect("core server stopped with an error");
}
