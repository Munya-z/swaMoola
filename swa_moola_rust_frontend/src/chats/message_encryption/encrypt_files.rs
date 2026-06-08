use base64::{Engine as _, engine::general_purpose::STANDARD};
use crate::interceptor::authenticated_fetch;
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce, aead::{Aead, KeyInit}};
use rand::RngCore;
use reqwest::Method;  
use leptos_router::NavigateOptions;

pub fn encrypt_raw_file_bytes(
    raw_file_bytes: &[u8], 
    file_secret_key: &[u8; 32]
) -> Result<(Vec<u8>, String), String> {
    let mut rng = rand::thread_rng();

    let mut nonce_bytes = [0u8; 12];
    rng.fill_bytes(&mut nonce_bytes);
    
    let key = Key::from_slice(file_secret_key);
    let cipher = ChaCha20Poly1305::new(key);
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let encrypted_bytes = cipher.encrypt(nonce, raw_file_bytes)
        .map_err(|e| format!("Symmetric encryption failed: {:?}", e))?;

    let nonce_base = STANDARD.encode(nonce_bytes);

    Ok((encrypted_bytes, nonce_base))
}

pub async fn upload_encrypted_file_to_storage(
    navigate: impl Fn(&str, NavigateOptions) + Clone + 'static, 
    encrypted_file_bytes: Vec<u8>) -> Result<String, String> 
    {
    
    let timestamp = web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now() as u64)
        .unwrap_or(0);
        
    let filename = format!("vault_file_{}.enc", timestamp);
    let url = format!("http://localhost:8000/api/upload/{}", filename);

    let response = authenticated_fetch(Method::POST, &url, navigate, Some(encrypted_file_bytes)).await; 
        
    match response{
        Ok(res)=> {
            if !res.status().is_success() {
                return Err(format!("Server returned error status: {}", res.status()));
            }
            
            let storage_url = res.text()
                .await
                .map_err(|e| format!("Failed to read response text: {:?}", e))?;
    
            Ok(storage_url)
        }
        Err(e)=>{
            Err(format!("Server rejected upload with code: {}", e))
        }
    }
}