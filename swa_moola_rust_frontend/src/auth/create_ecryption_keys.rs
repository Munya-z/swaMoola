use serde::{Deserialize, Serialize};
use web_sys::window;
use ml_kem::{MlKem768, kem::Kem, KeyExport}; 
use leptos::serde_json;
use base64::engine::{general_purpose::STANDARD as BASE64_STANDARD, Engine};

pub fn generate_hybrid_identities() -> (String, String) {
    use x25519_dalek::{StaticSecret, PublicKey};

    use rand::SeedableRng as SeedableRngV10; 
    use rand_core::SeedableRng as SeedableRngV6;   

    // 1. Get random bytes directly from the browser's WebCrypto via getrandom
    let mut x25519_seed = [0u8; 32];
    let mut mlkem_seed = [0u8; 32]; 
    
    let _ = getrandom::getrandom(&mut x25519_seed);
    let _ = getrandom::getrandom(&mut mlkem_seed);

    // 2. Initialize real Cryptographically Secure Pseudo-Random Number Generators (CSPRNG)
    let mut legacy_rng = rand::rngs::StdRng::from_seed(x25519_seed);
    let mut modern_rng = rand_chacha::ChaCha20Rng::from_seed(mlkem_seed);

    // 3. Classical Keys
    let x_private = StaticSecret::random_from_rng(&mut legacy_rng);
    let x_public = PublicKey::from(&x_private);

    // 4. Post-Quantum Keys
    let (pq_private, pq_public) = MlKem768::generate_keypair_from_rng(&mut modern_rng);

    let x_priv_bytes = x_private.to_bytes();
    let pq_priv_ref: &ml_kem::DecapsulationKey<MlKem768> = &pq_private;
    let pq_priv_bytes = pq_priv_ref.to_bytes(); 

    let pq_pub_ref: &ml_kem::EncapsulationKey<MlKem768> = &pq_public;
    let pq_pub_bytes = pq_pub_ref.to_bytes();

    let keys = SavedPrivateKeys {
        x25519_private_b64: BASE64_STANDARD.encode(x_priv_bytes),
        mlkem_private_b64: BASE64_STANDARD.encode(pq_priv_bytes),
    };

    // send the private keys to localstorage 
    let _ = save_private_keys_locally(&keys);

    // Return (X_pub_str, PQ_pub_str)
    (
        BASE64_STANDARD.encode(x_public.as_bytes()),
        BASE64_STANDARD.encode(pq_pub_bytes),
    )
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SavedPrivateKeys {
    pub x25519_private_b64: String,
    pub mlkem_private_b64: String,
}

pub struct RetrievedPrivateKeys {
    pub x25519_private: [u8; 32],
    pub mlkem_private: [u8; 64],
}

// Saves private keys to localStorage so they persist across browser tabs
pub fn save_private_keys_locally(keys: &SavedPrivateKeys) {
    if let Some(win) = window() {
        if let Some(storage) = win.local_storage().unwrap() {
            let serialized = serde_json::to_string(keys).unwrap();
            storage.set_item("e2ee_private_identities", &serialized).unwrap();
        }
    }
}

// Retrieves private keys from storage when your chat view initializes
pub fn load_private_keys_locally() -> Option<RetrievedPrivateKeys> {
    
    let storage = window()?.local_storage().ok()??;
    let serialized = storage.get_item("e2ee_private_identities").ok()??;
    
    let text_container: SavedPrivateKeys = serde_json::from_str(&serialized).ok()?;

    let mut x25519_private = [0u8; 32];
    let decoded_x = BASE64_STANDARD.decode(&text_container.x25519_private_b64).ok()?;
    if decoded_x.len() < 32 { return None; } // Guard against clipped payloads
    x25519_private.copy_from_slice(&decoded_x[..32]);

    let mut mlkem_private = [0u8; 64];
    let decoded_m = BASE64_STANDARD.decode(&text_container.mlkem_private_b64).ok()?;
    if decoded_m.len() < 64 { return None; } // Guard against clipped payloads
    mlkem_private.copy_from_slice(&decoded_m[..64]);

    Some(RetrievedPrivateKeys {
        x25519_private,
        mlkem_private,
    })
}



