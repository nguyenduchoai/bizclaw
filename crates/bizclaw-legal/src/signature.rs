use crate::contracts::Contract;

pub struct SignatureService {
    private_key: Option<String>,
}

impl SignatureService {
    pub fn new() -> Self {
        Self { private_key: None }
    }

    pub fn with_key(mut self, key: &str) -> Self {
        self.private_key = Some(key.to_string());
        self
    }

    pub fn sign(&self, contract: &Contract, _signature_data: &[u8]) -> anyhow::Result<String> {
        let contract_hash = self.hash_contract(contract)?;
        
        let signature = format!(
            "SIG-{}-{}",
            contract.id,
            &contract_hash[..8]
        );
        
        Ok(signature)
    }

    pub fn verify(&self, contract: &Contract, signature: &str) -> bool {
        let expected = self.sign(contract, &[]).unwrap_or_default();
        signature == expected
    }

    fn hash_contract(&self, contract: &Contract) -> anyhow::Result<String> {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        hasher.update(&contract.id);
        hasher.update(&contract.number);
        hasher.update(&contract.content);
        hasher.update(format!("{:?}", contract.signatures));
        
        let result = hasher.finalize();
        Ok(base16_encode(&result))
    }

    pub fn generate_digital_signature_request(&self, contract_id: &str) -> String {
        format!(
            "https://sign.example.vn/verify?contract={}&timestamp={}",
            contract_id,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        )
    }
}

fn base16_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect()
}

impl Default for SignatureService {
    fn default() -> Self {
        Self::new()
    }
}
