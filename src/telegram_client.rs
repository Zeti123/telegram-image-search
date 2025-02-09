use anyhow::{anyhow, Result};
use grammers_client::{types, grammers_tl_types};
use grammers_session::Session;
use grammers_tl_types::enums;
use grammers_tl_types::functions::channels;
use log::info;


pub struct LoginCredentials {
    pub api_id: i32,
    pub api_hash: String,
    pub phone_number: String,
    pub session_filename: String,
    pub confirmation_code_provider: Box<dyn Fn() -> String>
}

pub struct TelegramClient {
    login_credentials: LoginCredentials,
    client: Option<grammers_client::Client>
}

impl TelegramClient {
    pub fn new(login_credentials: LoginCredentials) -> Self {
        Self {
            login_credentials: login_credentials,
            client: None
        }
    }

    pub async fn connect_to_telegram(&mut self) -> Result<()> {
        log::debug!("connect_to_telegram({})", self.login_credentials.session_filename);

        let session = Session::load_file_or_create(&self.login_credentials.session_filename)?;
        let client = grammers_client::Client::connect(grammers_client::Config {
            session,
            api_id: self.login_credentials.api_id,
            api_hash: self.login_credentials.api_hash.clone(),
            params: Default::default(),
        }).await?;

        if !client.is_authorized().await? {
            log::info!("Additional authorization needed, asking user for confirmation code");

            let token = client.request_login_code(self.login_credentials.phone_number.as_str()).await?;
            let code = (self.login_credentials.confirmation_code_provider)();

            client.sign_in(&token, code.trim()).await?;
            client.session().save_to_file(&self.login_credentials.session_filename)?;
            log::info!("Session saved as {}", self.login_credentials.session_filename);
        }

        log::info!("User successfully logged in");

        self.client = Some(client);

        Ok(())
    }

    pub async fn find_channel_by_name(&self, channel_name: &str) -> Result<Option<types::Channel>> {
        let mut dialogs = self.get_client()?.iter_dialogs();

        while let Some(dialog) = dialogs.next().await? {
            if let types::Chat::Channel(channel) = dialog.chat {
                if channel.title() == channel_name {
                    return Ok(Some(channel));
                }
            }
        }

        Ok(None)
    }

    pub async fn find_channel_by_id(&self, id: i64) -> Result<Option<types::Channel>> {
        let mut dialogs = self.get_client()?.iter_dialogs();

        while let Some(dialog) = dialogs.next().await? {
            if let types::Chat::Channel(channel) = dialog.chat {
                if channel.id() == id {
                    return Ok(Some(channel));
                }
            }
        }

        Ok(None)
    }

    pub async fn create_channel(&self, channel_name: &str, about: &str) -> Result<i64> {
        log::debug!("create_channel({}, {})", channel_name, about);

        let client = self.get_client()?;

        let channel = client.invoke(&channels::CreateChannel {
            broadcast: false,
            megagroup: false,
            for_import: false,
            forum: false,
            title: channel_name.to_string(),
            about: about.to_string(),
            geo_point: None,
            address: None,
            ttl_period: None
        }).await?;

        match channel {
            enums::Updates::Updates(u) => {
                match &u.chats.first() {
                    Some(enums::Chat::Channel(channel)) => {
                        log::info!("Successfully created channel with id {}", channel.id);

                        Ok(channel.id)
                    },
                    _ => Err(anyhow!("Channel not created in response to create channel"))
                }
            }
            _ => Err(anyhow!("Channel not found in response to create channel"))
        }
    }

    pub async fn wait_for_message(&self, filter: Box<dyn Fn(&types::Message) -> bool>) -> Result<types::Message> {
        log::info!("Start to listen for messages");

        loop {
            if let grammers_client::Update::NewMessage(message) = self.get_client()?.next_update().await? {
                if filter(&message) {
                    info!("Received message with id {}", message.id());

                    return Ok(message);
                }
            }
        }
    }

    pub async fn wait_for_message_with_media(&self, filter: Box<dyn Fn(&types::Message) -> bool>)
        -> Result<(types::Message, Option<Vec<u8>>)> {
            let message = self.wait_for_message(filter).await?;

            let media = match message.media() {
                Some(media) => Some(self.download_media(media).await?),
                None => None
            };

            Ok((message, media))
    }

    pub async fn download_media(&self, media: types::Media) -> Result<Vec<u8>> {
        log::debug!("Downloading media");

        let mut file_bytes = Vec::new();
        let mut download = self.get_client()?.iter_download(&types::Downloadable::Media(media));

        while let Some(chunk) = download.next().await? {
            file_bytes.extend(chunk);
        }

        Ok(file_bytes)
    }

    pub async fn send_silent_response(&self, message: &types::Message, text: &str) -> Result<types::Message> {
        log::info!("Sending response to message with id {}", message.id());

        let message = message.reply(grammers_client::InputMessage::text(text).silent(true)).await?;

        Ok(message)
    }

    pub async fn forward_message(&self, target_chat: &types::Channel, message: &types::Message)
        -> Result<types::Message> {

        let messages = self.get_client()?.forward_messages(target_chat, &[message.id()], message.chat()).await?;

        if let Some(Some(message)) = messages.first() {
            log::info!("Message with id {} was forwarded to {}", message.id(), message.chat().name());
            return Ok(message.clone());
        }

        Err(anyhow!("Cannot forward message with id {}", message.id()))
    }

    fn get_client(&self) -> Result<&grammers_client::Client> {
        self.client.as_ref().ok_or_else(|| anyhow!("User not yet logged in, first log in"))
    }
}