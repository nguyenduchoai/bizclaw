//! Zalo encryption — AES-256-ECB for message encryption.
//! Based on reverse-engineered Zalo Web encryption protocol.

use aes::Aes256;
use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit, generic_array::GenericArray};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;
type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;

/// Encrypt data using AES-256-ECB (Zalo's message encryption).
pub fn encrypt_aes256(data: &[u8], key: &[u8; 32]) -> Vec<u8> {
    let cipher = Aes256::new(GenericArray::from_slice(key));

    // PKCS7 padding
    let block_size = 16;
    let padding_len = block_size - (data.len() % block_size);
    let mut padded = data.to_vec();
    padded.extend(std::iter::repeat_n(padding_len as u8, padding_len));

    // Encrypt each block
    let mut encrypted = Vec::with_capacity(padded.len());
    for chunk in padded.chunks(block_size) {
        let mut block = GenericArray::clone_from_slice(chunk);
        cipher.encrypt_block(&mut block);
        encrypted.extend_from_slice(&block);
    }

    encrypted
}

/// Decrypt data using AES-256-ECB.
pub fn decrypt_aes256(data: &[u8], key: &[u8; 32]) -> Vec<u8> {
    let cipher = Aes256::new(GenericArray::from_slice(key));

    let block_size = 16;
    let mut decrypted = Vec::with_capacity(data.len());

    for chunk in data.chunks(block_size) {
        if chunk.len() == block_size {
            let mut block = GenericArray::clone_from_slice(chunk);
            cipher.decrypt_block(&mut block);
            decrypted.extend_from_slice(&block);
        }
    }

    // Remove PKCS7 padding
    if let Some(&pad_len) = decrypted.last() {
        let pad_len = pad_len as usize;
        if pad_len <= block_size && pad_len <= decrypted.len() {
            let valid_padding = decrypted[decrypted.len() - pad_len..]
                .iter()
                .all(|&b| b == pad_len as u8);
            if valid_padding {
                decrypted.truncate(decrypted.len() - pad_len);
            }
        }
    }

    decrypted
}

/// Derive an encryption key from Zalo's zpw_enk.
pub fn derive_key(zpw_enk: &str) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(zpw_enk.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

/// Encrypt data using AES-CBC with PKCS7 padding and zero-IV.
/// Used for API param encryption like sendBankCard.
pub fn encode_aes_cbc_base64(data: &str, secret_key_b64: &str) -> Option<String> {
    use aes::cipher::{BlockEncryptMut, KeyIvInit};
    use cbc::cipher::block_padding::Pkcs7;

    let key = BASE64.decode(secret_key_b64).ok()?;
    let iv = [0u8; 16];

    let mut buf = vec![0u8; data.len() + 16];
    let pt_len = data.len();
    buf[..pt_len].copy_from_slice(data.as_bytes());

    let encrypted_len = if key.len() == 32 {
        let enc = Aes256CbcEnc::new_from_slices(&key, &iv).ok()?;
        enc.encrypt_padded_mut::<Pkcs7>(&mut buf, pt_len)
            .ok()?
            .len()
    } else if key.len() == 16 {
        let enc = Aes128CbcEnc::new_from_slices(&key, &iv).ok()?;
        enc.encrypt_padded_mut::<Pkcs7>(&mut buf, pt_len)
            .ok()?
            .len()
    } else {
        return None;
    };

    Some(BASE64.encode(&buf[..encrypted_len]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = derive_key("test_encryption_key_12345");
        let plaintext = b"Hello from BizClaw!";

        let encrypted = encrypt_aes256(plaintext, &key);
        let decrypted = decrypt_aes256(&encrypted, &key);

        assert_eq!(decrypted, plaintext);
    }
}

/// Decode Zalo WebSocket event data payloads (Ported from zca-js).
/// Supports encrypt types 0, 1, 2, 3 where 2 uses AES-GCM + zlib.
pub fn decode_event_data(
    data: &str,
    encrypt_type: i64,
    cipher_key_b64: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    use std::io::Read;

    if encrypt_type == 0 {
        return Ok(serde_json::from_str(data)?);
    }

    // Decode Base64
    let decoded_buffer = if encrypt_type == 1 {
        BASE64.decode(data)?
    } else {
        let decoded_uri = urlencoding::decode(data)?;
        BASE64.decode(decoded_uri.as_bytes())?
    };

    let decrypted_buffer = if encrypt_type != 1 {
        let cipher_key = cipher_key_b64
            .ok_or_else(|| anyhow::anyhow!("Missing cipher_key for encrypted payload"))?;
        let key_bytes = BASE64.decode(cipher_key)?;

        if decoded_buffer.len() < 48 {
            return Err(anyhow::anyhow!("Data too short for AES-GCM decryption"));
        }

        // Zalo AES-GCM structure: [IV 16 bytes] + [AAD 16 bytes] + [Ciphertext + Tag]
        let iv = &decoded_buffer[0..16];
        let aad = &decoded_buffer[16..32];
        let data_source = &decoded_buffer[32..];

        use aes_gcm::{
            Aes256Gcm, KeyInit,
            aead::{Aead, Payload},
        };
        let cipher = Aes256Gcm::new_from_slice(&key_bytes)?;

        let payload = Payload {
            msg: data_source,
            aad,
        };

        cipher
            .decrypt(iv.into(), payload)
            .map_err(|e| anyhow::anyhow!("AES-GCM Decrypt error: {:?}", e))?
    } else {
        decoded_buffer
    };

    let decompressed_buffer = if encrypt_type == 3 {
        decrypted_buffer
    } else {
        let mut deflater = flate2::read::ZlibDecoder::new(&decrypted_buffer[..]);
        let mut out = Vec::new();
        deflater.read_to_end(&mut out)?;
        out
    };

    let json_str = String::from_utf8(decompressed_buffer)?;
    Ok(serde_json::from_str(&json_str)?)
}

/// Implement ParamsEncryptor logic for initial handshake auth.
pub struct ParamsEncryptor {
    pub zcid: Option<String>,
    pub zcid_ext: String,
    pub enc_ver: String,
    pub encrypt_key: Option<String>,
}

impl ParamsEncryptor {
    pub fn new(type_val: i64, imei: &str, first_launch_time: i64) -> Self {
        use rand::RngCore;
        let mut rng = rand::thread_rng();
        let mut rand_bytes = [0u8; 6];
        rng.fill_bytes(&mut rand_bytes);
        let zcid_ext = hex::encode(rand_bytes);

        let mut encryptor = Self {
            zcid: None,
            zcid_ext,
            enc_ver: "v2".to_string(),
            encrypt_key: None,
        };
        encryptor.create_zcid(type_val, imei, first_launch_time);
        encryptor.create_encrypt_key(0);
        encryptor
    }

    fn create_zcid(&mut self, type_val: i64, imei: &str, first_launch_time: i64) {
        let msg = format!("{},{},{}", type_val, imei, first_launch_time);
        if let Some(encoded) = Self::encode_aes_hex("3FC4F0D2AB50057BCE0D90D9187A22B1", &msg) {
            self.zcid = Some(encoded);
        }
    }

    fn create_encrypt_key(&mut self, attempt: u8) -> bool {
        if self.zcid.is_none() || attempt >= 3 {
            return false;
        }

        use md5::{Digest, Md5};
        let mut hasher = Md5::new();
        hasher.update(&self.zcid_ext);
        let n_str = format!("{:X}", hasher.finalize());

        let zcid_clone = self.zcid.clone().unwrap();
        if self.process_and_store_key(&n_str, &zcid_clone) {
            return true;
        }

        self.create_encrypt_key(attempt + 1)
    }

    fn process_and_store_key(&mut self, e: &str, t: &str) -> bool {
        let even_e = Self::get_even_chars(e);
        let even_t = Self::get_even_chars(t);
        let odd_t = Self::get_odd_chars(t);

        if even_e.len() < 8 || even_t.len() < 12 || odd_t.len() < 12 {
            return false;
        }

        let mut final_key = String::new();
        final_key.push_str(&even_e[..8]);
        final_key.push_str(&even_t[..12]);
        let reversed_odd_t: String = odd_t.chars().rev().collect();
        final_key.push_str(&reversed_odd_t[..12]);

        self.encrypt_key = Some(final_key);
        true
    }

    fn get_even_chars(s: &str) -> String {
        s.chars()
            .enumerate()
            .filter(|(i, _)| i % 2 == 0)
            .map(|(_, c)| c)
            .collect()
    }

    fn get_odd_chars(s: &str) -> String {
        s.chars()
            .enumerate()
            .filter(|(i, _)| i % 2 != 0)
            .map(|(_, c)| c)
            .collect()
    }

    fn encode_aes_hex(key_str: &str, msg: &str) -> Option<String> {
        // Hex encoder for CBC
        use aes::cipher::{BlockEncryptMut, KeyIvInit};
        use cbc::cipher::block_padding::Pkcs7;

        let key = key_str.as_bytes();
        let iv = [0u8; 16];

        let mut buf = vec![0u8; msg.len() + 16];
        let pt_len = msg.len();
        buf[..pt_len].copy_from_slice(msg.as_bytes());

        let enc = Aes256CbcEnc::new_from_slices(key, &iv).ok()?;
        let encoded_len = enc
            .encrypt_padded_mut::<Pkcs7>(&mut buf, pt_len)
            .ok()?
            .len();

        Some(hex::encode_upper(&buf[..encoded_len]))
    }

    pub fn get_encrypt_key(&self) -> Option<&String> {
        self.encrypt_key.as_ref()
    }

    pub fn get_params(&self) -> std::collections::BTreeMap<&'static str, String> {
        let mut params = std::collections::BTreeMap::new();
        if let Some(zcid) = &self.zcid {
            params.insert("zcid", zcid.clone());
        }
        params.insert("zcid_ext", self.zcid_ext.clone());
        params.insert("enc_ver", self.enc_ver.clone());
        params
    }
}

/// Generate MD5 signature for API requests based on sorted param values.
pub fn get_sign_key(type_str: &str, params: &std::collections::BTreeMap<&str, String>) -> String {
    use md5::{Digest, Md5};
    let mut hasher = Md5::new();

    // Format: "zsecure" + type_str + values in alphabetical order of their keys.
    // Because we use BTreeMap, iterating it yields keys in alphabetical order naturally.
    let mut data_to_hash = format!("zsecure{}", type_str);
    for (_, value) in params {
        data_to_hash.push_str(value);
    }

    hasher.update(data_to_hash.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// AES-256-CBC with Base64 output (used for encoding params for outgoing HTTP API calls like sendMsg).
pub fn encode_aes_base64(secret_key_b64: &str, msg: &str) -> Option<String> {
    use aes::cipher::{BlockEncryptMut, KeyIvInit};
    use cbc::cipher::block_padding::Pkcs7;

    let key = BASE64.decode(secret_key_b64).ok()?;
    let iv = [0u8; 16];

    let mut buf = vec![0u8; msg.len() + 16];
    let pt_len = msg.len();
    buf[..pt_len].copy_from_slice(msg.as_bytes());

    let enc = Aes256CbcEnc::new_from_slices(&key, &iv).ok()?;
    let encoded_len = enc
        .encrypt_padded_mut::<Pkcs7>(&mut buf, pt_len)
        .ok()?
        .len();

    Some(BASE64.encode(&buf[..encoded_len]))
}

/// AES-256-CBC with Base64 output (used for decrypting API server responses).
pub fn decode_aes_base64(secret_key_b64: &str, data_b64: &str) -> Option<String> {
    use aes::cipher::{BlockDecryptMut, KeyIvInit};
    use cbc::cipher::block_padding::Pkcs7;

    let key = BASE64.decode(secret_key_b64).ok()?;
    let iv = [0u8; 16];
    let mut ciphertext = BASE64.decode(data_b64).ok()?;

    let dec = Aes256CbcDec::new_from_slices(&key, &iv).ok()?;
    let decrypted = dec.decrypt_padded_mut::<Pkcs7>(&mut ciphertext).ok()?;

    String::from_utf8(decrypted.to_vec()).ok()
}
