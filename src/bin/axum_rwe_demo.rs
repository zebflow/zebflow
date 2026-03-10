use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let showcase =
        zebflow::rwe::axum_demo::build_showcase_router().map_err(std::io::Error::other)?;
    let dx = zebflow::rwe::axum_demo::build_dx_test_router().map_err(std::io::Error::other)?;
    let app = showcase.merge(dx);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8787));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("Zebflow RWE showcase listening on http://{}", addr);
    println!("Routes: / (all hooks) | /blog (search + filter) | /todo (full todo app)");
    println!("DX Foundation: http://{}/dx-test", addr);
    axum::serve(listener, app).await?;
    Ok(())
}
