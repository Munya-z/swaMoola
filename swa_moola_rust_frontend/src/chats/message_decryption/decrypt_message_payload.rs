use leptos::{serde_json};
use ml_kem::Decapsulate;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce, aead::{Aead, KeyInit}};
use x25519_dalek::{StaticSecret, PublicKey as XPublicKey};
use ml_kem::ml_kem_768::{DecapsulationKey, Ciphertext};
use std::{error::Error};
use crate::chats::models::{SecretInnerPayload, Envelope};
use sha2::Sha256;
use sha2::Digest;

use crate::chats::models::InboundMessagePayload;


pub fn decrypt_message_with_fallback(
    payload: &InboundMessagePayload,
    my_x25519_private: &[u8; 32],
    my_mlkem_private: &[u8; 64], 
) -> Result<SecretInnerPayload, Box<dyn Error>> {
    
    for envelope in &payload.envelopes {
        
        if let Ok(inner_payload) = execute_envelope_decryption_track(
            &payload.ciphertext,
            &payload.nonce,
            envelope, 
            my_x25519_private,
            my_mlkem_private,
        ) {
            return Ok(inner_payload);
        }
    }

    Err("Cryptographic authentication failure. Key mismatch or payload tampered.".into())
}

fn execute_envelope_decryption_track(
    main_ciphertext_b64: &str,
    main_nonce_b64: &str,
    envelope: &Envelope,
    my_x25519_private: &[u8; 32],
    my_mlkem_private: &[u8; 64],
) -> Result<SecretInnerPayload, Box<dyn Error>> {
    let remote_x25519_bytes = STANDARD.decode(&envelope.ephemeral_x25519)?;
    let remote_pq_bytes = STANDARD.decode(&envelope.pq_ciphertext)?;
    let wrapped_master_key = STANDARD.decode(&envelope.encrypted_master_key)?;

    let my_static_secret = StaticSecret::from(*my_x25519_private);
    let mut x25519_pub_bytes = [0u8; 32];
    x25519_pub_bytes.copy_from_slice(&remote_x25519_bytes[..32]);
    let remote_x25519_pub = XPublicKey::from(x25519_pub_bytes);
    let x25519_shared = my_static_secret.diffie_hellman(&remote_x25519_pub);
    let x25519_shared_bytes: [u8; 32] = x25519_shared.to_bytes();

    let seed_bytes: [u8; 64] = my_mlkem_private[..64].try_into().map_err(|_| "Invalid seed length")?;
    let seed = hybrid_array::Array::from(seed_bytes);

    let my_decaps_key = DecapsulationKey::from(seed);

    let pq_ciphertext_input = Ciphertext::try_from(remote_pq_bytes.as_slice())?;
    let ml_kem_shared = my_decaps_key.decapsulate(&pq_ciphertext_input);

    let mut combined_secrets = Vec::new();
    combined_secrets.extend_from_slice(&x25519_shared_bytes);
    combined_secrets.extend_from_slice(ml_kem_shared.as_slice());
    let derived_kek = Sha256::digest(&combined_secrets);
    
    let kek_cipher = ChaCha20Poly1305::new(Key::from_slice(&derived_kek));
    let fixed_envelope_nonce = Nonce::from_slice(&[0u8; 12]);

    let decrypted_key_bytes = kek_cipher.decrypt(fixed_envelope_nonce, wrapped_master_key.as_slice())
    .map_err(|e| {
        format!("Envelope decryption failed: {:?}", e)})?;
    
    let mut master_msg_key = [0u8; 32];
    master_msg_key.copy_from_slice(&decrypted_key_bytes[..32]);

    let raw_ciphertext = STANDARD.decode(main_ciphertext_b64)?;
    let raw_nonce = STANDARD.decode(main_nonce_b64)?;

    let cipher = ChaCha20Poly1305::new(Key::from_slice(&master_msg_key));
    let nonce = Nonce::from_slice(&raw_nonce);
    let decrypted_bytes = cipher.decrypt(nonce, raw_ciphertext.as_slice())
    .map_err(|e|{
        format!("Core payload decryption failed: {:?}", e)
    })?;

    let inner_data: SecretInnerPayload = serde_json::from_slice(&decrypted_bytes)?;
    Ok(inner_data)
}
