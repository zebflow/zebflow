use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = zebflow::rwe::axum_demo::build_demo_router().map_err(std::io::Error::other)?;
    let addr = SocketAddr::from(([127, 0, 0, 1], 8787));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("Zebflow Axum RWE demo listening on http://{}", addr);
    println!(
        "Try routes: / /rwe/comprehensive /rwe/dashboard /rwe/frontpage /rwe/lab /showcase /recycling /todo /list-hydration /state-sharing /blog /blog/post-a /blog/composed"
    );
    axum::serve(listener, app).await?;
    Ok(())
}
