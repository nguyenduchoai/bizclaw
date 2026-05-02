use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use serde_json::json;

use crate::contracts::{Contract, ContractManager};
use crate::compliance::ComplianceChecker;
use crate::templates::TemplateLibrary;
use crate::signature::SignatureService;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalMessage {
    pub intent: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalResponse {
    pub success: bool,
    pub data: serde_json::Value,
    pub error: Option<String>,
}

impl LegalResponse {
    pub fn success(data: impl Serialize) -> Self {
        Self {
            success: true,
            data: serde_json::to_value(data).unwrap_or(json!({})),
            error: None,
        }
    }

    pub fn error(msg: &str) -> Self {
        Self {
            success: false,
            data: json!({}),
            error: Some(msg.to_string()),
        }
    }
}

pub struct LegalAgent {
    contract_manager: Arc<RwLock<ContractManager>>,
    compliance_checker: Arc<RwLock<ComplianceChecker>>,
    template_library: Arc<TemplateLibrary>,
    signature_service: Arc<SignatureService>,
}

impl LegalAgent {
    pub fn new() -> Self {
        Self {
            contract_manager: Arc::new(RwLock::new(ContractManager::new())),
            compliance_checker: Arc::new(RwLock::new(ComplianceChecker::new())),
            template_library: Arc::new(TemplateLibrary::new()),
            signature_service: Arc::new(SignatureService::new()),
        }
    }

    pub async fn create_contract(&self, contract: Contract) -> Result<Contract> {
        info!("Creating contract");
        let mut manager = self.contract_manager.write().await;
        manager.create_contract(contract).await
    }

    pub async fn generate_from_template(
        &self,
        template_id: &str,
        parties: &str,
        terms: serde_json::Value,
    ) -> Result<Contract> {
        let contract = self.template_library.fill_template(template_id, parties, terms)?;
        self.create_contract(contract).await
    }

    pub async fn check_compliance(&self, contract: &Contract) -> Result<Vec<crate::compliance::ComplianceCheck>> {
        info!("Checking compliance for contract");
        let mut checker = self.compliance_checker.write().await;
        checker.check_contract(contract).await
    }

    pub async fn sign_contract(&self, contract_id: &str, signature_data: &[u8]) -> Result<String> {
        let manager = self.contract_manager.read().await;
        let contract = manager.get_contract(contract_id).await?;
        if let Some(contract) = contract {
            let signature = self.signature_service.sign(contract, signature_data)?;
            Ok(signature)
        } else {
            Err(anyhow::anyhow!("Contract not found"))
        }
    }

    pub async fn process(&self, message: LegalMessage) -> Result<LegalResponse> {
        debug!("Processing legal message: {:?}", message.intent);

        match message.intent.as_str() {
            "create_contract" => {
                let contract: Contract = serde_json::from_value(message.payload)?;
                let created = self.create_contract(contract).await?;
                Ok(LegalResponse::success(created))
            }
            "generate_contract" => {
                let template_id = message.payload.get("template_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let parties = message.payload.get("parties")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let terms = message.payload.get("terms").cloned().unwrap_or(json!({}));
                let contract = self.generate_from_template(template_id, parties, terms).await?;
                Ok(LegalResponse::success(contract))
            }
            "check_compliance" => {
                let contract: Contract = serde_json::from_value(message.payload.clone())?;
                let checks = self.check_compliance(&contract).await?;
                Ok(LegalResponse::success(checks))
            }
            "sign_contract" => {
                let contract_id = message.payload.get("contract_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let signature_data: Vec<u8> = message.payload.get("signature_data")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();
                let signature = self.sign_contract(contract_id, &signature_data).await?;
                Ok(LegalResponse::success(json!({"signature": signature})))
            }
            "list_templates" => {
                let templates = self.template_library.list_templates();
                Ok(LegalResponse::success(templates))
            }
            _ => Ok(LegalResponse::error("Unknown intent")),
        }
    }
}

impl Default for LegalAgent {
    fn default() -> Self {
        Self::new()
    }
}
