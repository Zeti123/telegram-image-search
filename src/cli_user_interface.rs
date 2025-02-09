use anyhow::Result;
use std::io;
use std::io::{stdin, Write};

mod file_encryptor;

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct UserData {
    pub api_id: i32,
    pub api_hash: String,
    pub phone_number: String,
    pub session_filename: String,
    pub channel_name: String
}

pub fn ask_user_for_data() -> UserData {
    log::debug!("Asking user for data");

    println!("Welcome to image-search");

    loop {
        println!("Would you like to load data from encrypted file? (if you use image-search first time you need to enter data manually) [y/n]");

        match read_string_untill_success().to_lowercase().chars().nth(0).unwrap_or_default() {
            'y' => {
                match load_data_from_file() {
                    Ok(data) => return data,
                    Err(e) => println!("Cannot load file, error: {}", e)
                }
            },
            'n' => return enter_data_manually(),
            _ => println!("You need to enter y or n")
        }
    }
}

pub fn get_verification_code() -> String {
    println!("Enter veryfication code:");

    read_string_untill_success()
}

fn load_data_from_file() -> Result<UserData> {
    print!("Login: ");
    io::stdout().flush().unwrap();
    let login = read_string_untill_success();
    print!("Password: ");
    io::stdout().flush().unwrap();
    let password = read_password_untill_success();

    log::info!("Loading user data from file {}.isuser", login);

    let filename = std::format!("{}.isuser", login);
    Ok(serde_json::from_str::<UserData>(&file_encryptor::decrypt_and_load_file(&filename, &password)?)?)
}

fn enter_data_manually()  -> UserData {

    log::info!("Entering user data manualny");

    let mut user_data = UserData::default();

    print!("Enter your username: ");
    io::stdout().flush().unwrap();
    let username = read_string_untill_success();
    user_data.session_filename = format!("{}.session", &username);

    print!("Enter your api_id: ");
    io::stdout().flush().unwrap();
    user_data.api_id = read_int_untill_success();

    print!("Enter your api_hash: ");
    io::stdout().flush().unwrap();
    user_data.api_hash = read_string_untill_success();

    print!("Enter your phone number with region code e.g. +48123456789: ");
    io::stdout().flush().unwrap();
    user_data.phone_number = read_string_untill_success()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();

    print!("Enter the name of the channel where you want the program to send messages: ");
    io::stdout().flush().unwrap();
    user_data.channel_name = read_string_untill_success();

    loop {
        println!("Would you like to save your data in encrypted file? [y/n]");

        match read_string_untill_success().to_lowercase().chars().nth(0).unwrap_or_default() {
            'y' => {
                match save_data_to_file(&username, &user_data) {
                    Ok(()) => {
                        println!("Data successfully saved to file");
                        break;
                    }
                    Err(e) => {
                        log::warn!("Error while saving file: {}", e);
                        println!("Error while saving file: {}", e)
                    }
                }
            }
            'n' => break,
            _ => println!("You need to enter y or n")
        }
    }

    user_data
}

fn save_data_to_file(username :&str, user_data: &UserData) -> Result<()>{
    print!("Enter new password: ");
    io::stdout().flush().unwrap();
    let password = read_password_untill_success();

    let filename = std::format!("{}.isuser", username);

    let user_data_json = serde_json::to_string(user_data)?;

    file_encryptor::encrypt_and_save_file(&user_data_json, &filename, &password)
}

fn read_string_untill_success() -> String {
    let mut buff: String = String::new();

    while let Err(error) = stdin().read_line(&mut buff) {
        println!("{}, try again", error);
    }

    buff.chars().filter(|c| !char::is_whitespace(*c)).collect()
}

fn read_password_untill_success() -> String {
    loop {
        match rpassword::read_password() {
            Ok(password) => return password,
            Err(error) => println!("{}, try again", error)
        }
    }
}

fn read_int_untill_success() -> i32 {
    loop {
        let str = read_string_untill_success();
        match str.parse::<i32>() {
            Ok(int) => return int,
            Err(error) => {
                println!("{}, try again", error);
                continue
            }
        }
    }
}