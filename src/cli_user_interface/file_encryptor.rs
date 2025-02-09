use aes_gcm::{Aes256Gcm, Nonce};
use aes_gcm::aead::{rand_core, Aead, AeadCore, KeyInit};
use anyhow::Result;
use argon2::{Argon2, PasswordHasher, password_hash};
use argon2::password_hash::Salt;
use base64::Engine;
use base64::engine::general_purpose;
use rand::Rng;
use std::fs;
use std::io::Write;


pub fn encrypt_and_save_file(input: &str, output_path: &str, password: &str) -> Result<()>{
    let salt: [u8; 16] = rand::rng().random();
    let nonce = Aes256Gcm::generate_nonce(rand_core::OsRng::default());
    
    let key = derive_key_from_password(password, &salt)
        .map_err(|e| anyhow::anyhow!(e))?;

    let cipher = Aes256Gcm::new_from_slice(&key)?;
    
    let encrypted_data = cipher.encrypt(&nonce, input.as_bytes())
        .map_err(|e| anyhow::anyhow!(e))?;
    
    let mut output_file = fs::File::create(output_path)?;
    output_file.write_all(&salt)?;
    output_file.write_all(nonce.as_slice())?;
    output_file.write_all(&encrypted_data)?;
    
    log::info!("Encrypted file saved as {}", output_path);

    Ok(())
}

pub fn decrypt_and_load_file(input_path: &str, password: &str) -> Result<String> {
    let file_content = fs::read(input_path)?;

    let salt = &file_content[..16];
    let nonce = &file_content[16..28];
    let encrypted_data = &file_content[28..];
    
    let key = derive_key_from_password(password, salt)
        .map_err(|e| anyhow::anyhow!(e))?;
    let cipher = Aes256Gcm::new_from_slice(&key)?;
    
    let decrypted_data = cipher.decrypt(Nonce::from_slice(nonce), encrypted_data.as_ref())
        .map_err(|e| anyhow::anyhow!(e))?;

    log::info!("Succesfully decrypted file {}", input_path);
    
    Ok(String::from_utf8(decrypted_data)?)
}

fn derive_key_from_password(password: &str, salt: &[u8]) -> Result<[u8; 32], password_hash::Error> {
    let argon2 = Argon2::default();
    let b64_salt = general_purpose::STANDARD_NO_PAD.encode(&salt);
    let salt = Salt::from_b64(b64_salt.as_str())?;
    let hash = argon2.hash_password(password.as_bytes(), salt)?;
    let hash = hash.hash.expect("Unable to get hash");
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash.as_bytes()[..32]);

    Ok(key)
}