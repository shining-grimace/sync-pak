#[tokio::main]
async fn main() -> std::process::ExitCode {
    match sync_pak::provider_probe::run_from_environment().await {
        Ok(()) => {
            println!("Provider probe completed successfully.");
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Provider probe failed: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
