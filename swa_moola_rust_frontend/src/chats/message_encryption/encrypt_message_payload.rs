use base64::{Engine as _, engine::general_purpose::STANDARD};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce, aead::{Aead, KeyInit}};
use x25519_dalek::{EphemeralSecret, PublicKey as XPublicKey};
use rand::RngCore;
use leptos::serde_json; 
use ml_kem::ml_kem_768::{EncapsulationKey,Ciphertext};
use uuid::Uuid;
use sha2::{Digest, Sha256};
use crate::chats::models::{ SecretInnerPayload, OutboundMessagePayload, Envelope};
use crate::chats::models::UserPublicKeys;

fn encrypt_person_payload(
    x25519: [u8; 32],     
    mlkem: [u8; 1184],
    master_msg_key: &[u8; 32],
)-> Envelope{
    let mut rng = rand::thread_rng();

    let mut user_seed = [0u8; 32];
    rng.fill_bytes(&mut user_seed);
    let user_seed_array = (&user_seed[..])
    .try_into()
    .expect("Failed to convert user seed to hybrid array");

    let rx_pub = XPublicKey::from(x25519);
    let user_ephemeral_secret = EphemeralSecret::random_from_rng(&mut rng);
    let user_ephemeral_public = XPublicKey::from(&user_ephemeral_secret);
    let user_x25519_shared = user_ephemeral_secret.diffie_hellman(&rx_pub);
    let user_x25519_shared_bytes: [u8; 32] = user_x25519_shared.to_bytes();

    let user_key_ref: &[u8; 1184]= (&mlkem[..])
    .try_into()
    .expect("Invalid recipient ML-KEM public key array structure length");
    let user_ek = EncapsulationKey::new(user_key_ref.into())
    .expect("Invalid underlying ML-KEM-768 public key layout components");
     let (user_ml_kem_ciphertext, user_ml_kem_shared): (Ciphertext, _) = 
        user_ek.encapsulate_deterministic(user_seed_array);

    let mut user_combined_secrets = Vec::new();
    user_combined_secrets.extend_from_slice(&user_x25519_shared_bytes);
    user_combined_secrets.extend_from_slice(user_ml_kem_shared.as_slice());
    let user_kek = Sha256::digest(&user_combined_secrets);

    let user_kek_cipher = ChaCha20Poly1305::new(Key::from_slice(&user_kek));
    let fixed_envelope_nonce = Nonce::from_slice(&[0u8; 12]); 
    let user_encrypted_key = user_kek_cipher.encrypt(fixed_envelope_nonce, master_msg_key.as_slice())
        .expect("Recipient key wrapping failed");
    
    let user_envelope = Envelope {
        ephemeral_x25519: STANDARD.encode(user_ephemeral_public.as_bytes()),
        pq_ciphertext: STANDARD.encode(user_ml_kem_ciphertext.as_slice()),
        encrypted_master_key: STANDARD.encode(user_encrypted_key),
    };

    user_envelope
} 

pub fn prepare_full_payload(
    inner_data: SecretInnerPayload,
    sender_x25519: [u8; 32],       
    sender_mlkem: [u8; 1184],      
    recipients: Vec<UserPublicKeys>, 
    recipient_id: Uuid,
) -> OutboundMessagePayload {

    let mut rng = rand::thread_rng();
    let mut master_msg_key = [0u8; 32];
    rng.fill_bytes(&mut master_msg_key);

    let mut msg_nonce_bytes = [0u8; 12];
    rng.fill_bytes(&mut msg_nonce_bytes);
    

    let serialized_inner = serde_json::to_vec(&inner_data)
        .expect("Failed to serialize message data body context");
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&master_msg_key));
    let nonce = Nonce::from_slice(&msg_nonce_bytes);
    let encrypted_bytes = cipher.encrypt(nonce, serialized_inner.as_slice())
        .expect("Envelope message data encryption error");

    let mut chat_envelopes = Vec::new();


    for  UserPublicKeys { x25519, mlkem } in recipients {
        let envelope = encrypt_person_payload(x25519, mlkem, &master_msg_key);

        chat_envelopes.push(envelope);
    }

    let sender_envelope = encrypt_person_payload(sender_x25519, sender_mlkem, &master_msg_key);
    chat_envelopes.push(sender_envelope);
    
    OutboundMessagePayload {
        recipient_id,
        ciphertext: STANDARD.encode(encrypted_bytes),
        nonce: STANDARD.encode(msg_nonce_bytes),
        envelopes: chat_envelopes,
    }
}