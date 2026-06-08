use base64::{Engine as _, engine::general_purpose::STANDARD};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce, aead::{Aead, KeyInit}};
use x25519_dalek::{EphemeralSecret, PublicKey as XPublicKey};
use rand::RngCore;
use leptos::serde_json; 
use ml_kem::ml_kem_768::{EncapsulationKey,Ciphertext};
use uuid::Uuid;
use sha2::{Digest, Sha256};
use crate::chats::models::{ SecretInnerPayload, OutboundMessagePayload, Envelope};

pub fn prepare_full_payload(
    inner_data: SecretInnerPayload,
    sender_x25519: [u8; 32],       
    sender_mlkem: [u8; 1184],      
    recipient_x25519: [u8; 32],     
    recipient_mlkem: [u8; 1184],    
    recipient_id: Uuid,
) -> OutboundMessagePayload {
    let mut rng = rand::thread_rng();

    let mut r_seed = [0u8; 32];
    let mut s_seed = [0u8; 32];
    rng.fill_bytes(&mut r_seed);
    rng.fill_bytes(&mut s_seed);
    let r_seed_array = hybrid_array::Array::<u8, hybrid_array::sizes::U32>::from_slice(&r_seed);
    let s_seed_array = hybrid_array::Array::<u8, hybrid_array::sizes::U32>::from_slice(&s_seed);

    let mut master_msg_key = [0u8; 32];
    rng.fill_bytes(&mut master_msg_key);

    
    let mut msg_nonce_bytes = [0u8; 12];
    rng.fill_bytes(&mut msg_nonce_bytes);
    

    let rx_pub = XPublicKey::from(recipient_x25519);
    let r_ephemeral_secret = EphemeralSecret::random_from_rng(&mut rng);
    let r_ephemeral_public = XPublicKey::from(&r_ephemeral_secret);
    let r_x25519_shared = r_ephemeral_secret.diffie_hellman(&rx_pub);
    let r_x25519_shared_bytes: [u8; 32] = r_x25519_shared.to_bytes();

    let r_key_ref: &[u8; 1184]= (&recipient_mlkem[..])
    .try_into()
    .expect("Invalid recipient ML-KEM public key array structure length");
    let r_ek = EncapsulationKey::new(r_key_ref.into())
    .expect("Invalid underlying ML-KEM-768 public key layout components");
     let (r_ml_kem_ciphertext, r_ml_kem_shared): (Ciphertext, _) = 
        r_ek.encapsulate_deterministic(r_seed_array);

    let sx_pub = XPublicKey::from(sender_x25519);
    let s_ephemeral_secret = EphemeralSecret::random_from_rng(&mut rng);
    let s_ephemeral_public = XPublicKey::from(&s_ephemeral_secret);
    let s_x25519_shared = s_ephemeral_secret.diffie_hellman(&sx_pub);
    let s_x25519_shared_bytes: [u8; 32] = s_x25519_shared.to_bytes();

    let s_key_ref: &[u8; 1184] = (&sender_mlkem[..])
    .try_into()
    .expect("Invalid sender ML-KEM public key array structure length");
    let s_ek = EncapsulationKey::new(s_key_ref.into())
    .expect("Invalid underlying ML-KEM-768 public key layout components");
    let (s_ml_kem_ciphertext, s_ml_kem_shared): (Ciphertext, _)  = s_ek.encapsulate_deterministic(s_seed_array);

    let mut r_combined_secrets = Vec::new();
    r_combined_secrets.extend_from_slice(&r_x25519_shared_bytes);
    r_combined_secrets.extend_from_slice(r_ml_kem_shared.as_slice());
    let r_kek = Sha256::digest(&r_combined_secrets);

    let mut s_combined_secrets = Vec::new();
    s_combined_secrets.extend_from_slice(&s_x25519_shared_bytes);
    s_combined_secrets.extend_from_slice(s_ml_kem_shared.as_slice());
    let s_kek = Sha256::digest(&s_combined_secrets);
   
    let r_kek_cipher = ChaCha20Poly1305::new(Key::from_slice(&r_kek));
    let fixed_envelope_nonce = Nonce::from_slice(&[0u8; 12]); // Constant nonce is safe for single-use ephemeral keys
    let r_encrypted_key = r_kek_cipher.encrypt(fixed_envelope_nonce, master_msg_key.as_slice())
        .expect("Recipient key wrapping failed");
    
    let s_kek_cipher = ChaCha20Poly1305::new(Key::from_slice(&s_kek));
    let s_encrypted_key = s_kek_cipher.encrypt(fixed_envelope_nonce, master_msg_key.as_slice())
        .expect("Sender key wrapping failed");

    let serialized_inner = serde_json::to_vec(&inner_data)
        .expect("Failed to serialize message data body context");
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&master_msg_key));
    let nonce = Nonce::from_slice(&msg_nonce_bytes);
    let encrypted_bytes = cipher.encrypt(nonce, serialized_inner.as_slice())
        .expect("Envelope message data encryption error");
        
    let r_envelope = Envelope {
        ephemeral_x25519: STANDARD.encode(r_ephemeral_public.as_bytes()),
        pq_ciphertext: STANDARD.encode(r_ml_kem_ciphertext.as_slice()),
        encrypted_master_key: STANDARD.encode(r_encrypted_key),
    };

    let s_envelope = Envelope {
        ephemeral_x25519: STANDARD.encode(s_ephemeral_public.as_bytes()),
        pq_ciphertext: STANDARD.encode(s_ml_kem_ciphertext.as_slice()),
        encrypted_master_key: STANDARD.encode(s_encrypted_key),
    };

    OutboundMessagePayload {
        recipient_id,
        ciphertext: STANDARD.encode(encrypted_bytes),
        nonce: STANDARD.encode(msg_nonce_bytes),
        s_envelope,
        r_envelope,
    }
}

