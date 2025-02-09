use env_logger::Builder;
use log::LevelFilter;
use std::fs;
use std::io::Write;
use tokio;

mod telegram_client;
mod telegram_service;
mod cli_user_interface;


fn init_logger() {
    let target = Box::new(fs::File::create("log.txt").expect("Can't create log file"));

    Builder::new()
        .target(env_logger::Target::Pipe(target))
        .format(|buf, record| {
            writeln!(
                buf,
                "[{} {} {}:{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                record.level(),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.args()
            )

        })
        .filter(Some(module_path!()), LevelFilter::Debug)
        .init();

    log::info!("Log initialized");
}

#[tokio::main]
async fn main() {
    let channel_about = "";

    init_logger();
    let user_data = cli_user_interface::ask_user_for_data();

    let telegram_login_credentials = telegram_client::LoginCredentials {
        api_id: user_data.api_id,
        api_hash: user_data.api_hash,
        phone_number: user_data.phone_number,
        session_filename: user_data.session_filename,
        confirmation_code_provider: Box::new(cli_user_interface::get_verification_code)
    };

    let mut telegram_service = telegram_service::TelegramService::new(telegram_login_credentials);

    telegram_service.run_telegram_service(&user_data.channel_name, &channel_about).await;
}