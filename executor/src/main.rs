#[tokio::main]
async fn main() {
    if mtmpg_executor::service::run().await.is_err() {
        eprintln!("executor failed to start or stopped unexpectedly");
        std::process::exit(78);
    }
}
