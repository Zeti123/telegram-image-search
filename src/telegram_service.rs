use anyhow::{anyhow, Result};
use grammers_client::types::{Media, Message, Channel};
use std::error::Error;
use tesseract::Tesseract;
use tokio;
use tokio::time::{sleep, Duration};

use crate::telegram_client::{self, TelegramClient};

pub struct TelegramService {
    telegram_client: TelegramClient
}

impl TelegramService {
    pub fn new(login_credentials: telegram_client::LoginCredentials) -> Self {
        Self {
            telegram_client: telegram_client::TelegramClient::new(login_credentials)
        }
    }

    pub async fn run_telegram_service(&mut self, channel_name: &str, channel_about: &str) {
        let mut retries: u32 = 0;
        loop {
            match self.telegram_client.connect_to_telegram().await {
                Ok(()) => {
                    /* println istead of log besause user should be able to see that application works
                    * without need to check logs, same for connection lost.
                    */
                    println!("Succesfully connected to telegram, start to listen for messages. All application logs will be saved in log.txt");

                    retries = 0;
                    let channel = self.try_to_create_channel_untill_success(channel_name, channel_about).await;
                    let result = self.start_receive_messages(channel).await;

                    if let Err(e) = result {
                        log::info!("Error occurred, trying to reconnect {}", e);
                        continue;
                    }
                }
                Err(e) => {
                    let retry_sec = 2u64.pow(retries);
                    println!("Lost connection reason: {}. Retry in {}s", e, retry_sec);
                    log::warn!("Lost connection reason: {}. Retry in {}s", e, retry_sec);

                    sleep(Duration::from_secs(retry_sec)).await;
                    retries = (retries + 1).max(7);
                }
            }
        }
    }

    async fn try_to_create_channel_untill_success(&self, channel_name: &str, about: &str) -> Channel {
        loop {
            let result = self.create_channel_if_doesnt_exist( channel_name, about).await;

            if let Err(e) = &result {
                log::error!("Cannot create channel reason: {}", e)
            }

            if let Ok(channel) = result {
                return channel;
            }
        }
    }

    async fn create_channel_if_doesnt_exist(&self, channel_name: &str, about: &str)
        -> Result<Channel, Box<dyn Error>> {
            Ok(match self.telegram_client.find_channel_by_name(channel_name).await? {
                Some(id) => { log::info!("Channel already exists, skipping creation"); id },
                None =>  {
                    let id = self.telegram_client.create_channel(channel_name, about).await?;
                    self.telegram_client.find_channel_by_id(id).await?.ok_or("Cannot find newly created channel")?
                }
            })
    }

    async fn start_receive_messages(&self, channel: Channel) -> Result<()> {
        let channel_id = channel.id();
        loop {
            let (message, media) = self.telegram_client.wait_for_message_with_media(
                Box::new(move |msg| { Self::is_image_media_present(msg) && msg.chat().id() != channel_id } )
            ).await?;

            match Self::get_text_from_message(&media.unwrap()) {
                Ok(Some(text)) => {
                    let message = self.telegram_client.forward_message(&channel, &message).await?;
                    self.telegram_client.send_silent_response(&message, text.as_str()).await?;

                    log::info!("Message with id {} from chat {}, from user {} succesfully forwarded to {}", message.id(),
                        message.chat().name(), message.post_author().unwrap_or("<Unknown>"), channel.title());
                },
                Ok(None) => log::info!("No test found on image, skipping this message"),
                Err(e) => log::info!("Error while reading text from image {}", e)
            }
        }
    }

    fn is_image_media_present(message: &Message) -> bool {
        if let Some(media) = message.media() {
            if let Media::Photo(_) = media {
                return true;
            }
        }

        return false;
    }

    fn get_text_from_message(image_mem: &[u8]) -> Result<Option<String>> {
        match Tesseract::new(None, Some("eng")) {
            Ok(tesseract) => {
                let mut tesseract = tesseract.set_image_from_mem(&image_mem)?.recognize()?;

                if tesseract.mean_text_conf() < 10 {
                    return Ok(None);
                }

                return Ok(Some(tesseract.get_text()?));
            },
            Err(e) => return Err(anyhow!("Error while creating tesseract: {}", e))
        }
    }
}