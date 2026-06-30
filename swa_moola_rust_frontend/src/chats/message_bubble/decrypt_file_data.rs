use chacha20poly1305::{Nonce, KeyInit, Key, aead::Aead};
use chacha20poly1305::ChaCha20Poly1305;
use base64::{Engine as _, engine::general_purpose::STANDARD};

pub fn decrypt_raw_file_bytes(
    encrypted_bytes: &[u8],
    nonce_base64: &str,
    file_secret_key: &[u8; 32]
) -> Result<Vec<u8>, String> {

    let first_decode = STANDARD.decode(nonce_base64)
        .map_err(|e| format!("First Base64 decode failed: {:?}", e))?;
        
    let intermediary_str = String::from_utf8(first_decode)
        .map_err(|e| format!("Failed to convert intermediary bytes to string: {:?}", e))?;

    let real_nonce_bytes = STANDARD.decode(&intermediary_str)
        .map_err(|e| format!("Second Base64 decode failed. Is this an old single-encoded asset?: {:?}", e))?;

    let key = Key::from_slice(file_secret_key);
    let cipher = ChaCha20Poly1305::new(key);
    let nonce = Nonce::from_slice(&real_nonce_bytes);
    let decrypted_bytes = cipher.decrypt(nonce, encrypted_bytes)
        .map_err(|e|{
            format!("Symmetric decryption failed: {:?}", e)  
        } )?;
    Ok(decrypted_bytes)
}

pub async fn get_encrypted_bytes_from_server(
url: String
)->Option<Vec<u8>>{
    let clean_url = if url.contains("127.0.0") {
        let file_name = url.replace("http://127.0.0", "");
        format!("/view-files/{}", file_name)
    } else if !url.starts_with("/view-files/") && !url.starts_with("http") {
        format!("/view-files/{}", url)
    } else {
        url
    };

    let _ = dotenvy::dotenv();
    let base_url = std::env::var("BACKEND_WS_URL")
    .unwrap_or_else(|_| "http://localhost:8000".to_string()); 

    let absolute_url = format!("{base_url}{}", clean_url);
    let client = reqwest::Client::new();
    let response = client.get(&absolute_url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }

    let bytes = response.bytes().await.ok()?;
    Some(bytes.to_vec())
}