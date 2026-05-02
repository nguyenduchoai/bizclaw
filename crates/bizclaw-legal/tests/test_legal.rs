#[cfg(test)]
mod tests {
    use bizclaw_legal::{LegalAgent, Contract, ContractType, Party};

    #[tokio::test]
    async fn test_create_contract() {
        let agent = LegalAgent::new();
        
        let contract = Contract::new(ContractType::Service, "Hợp đồng dịch vụ test");
        
        let result = agent.create_contract(contract).await;
        assert!(result.is_ok());
        
        let created = result.unwrap();
        assert_eq!(created.contract_type, ContractType::Service);
    }

    #[test]
    fn test_contract_creation() {
        let contract = Contract::new(ContractType::Service, "Test Contract");
        
        assert!(!contract.id.is_empty());
        assert!(contract.number.starts_with("HD-"));
        assert_eq!(contract.status, bizclaw_legal::ContractStatus::Draft);
    }

    #[test]
    fn test_contract_party() {
        let mut contract = Contract::new(ContractType::Service, "Test");
        
        contract.add_party(Party {
            id: "p1".to_string(),
            name: "Party A".to_string(),
            role: bizclaw_legal::PartyRole::Customer,
            address: None,
            representative: None,
            tax_code: None,
            email: None,
        });
        
        assert_eq!(contract.parties.len(), 1);
    }

    #[test]
    fn test_contract_signing() {
        let mut contract = Contract::new(ContractType::Service, "Test");
        
        contract.add_party(Party {
            id: "p1".to_string(),
            name: "Party A".to_string(),
            role: bizclaw_legal::PartyRole::Customer,
            address: None,
            representative: None,
            tax_code: None,
            email: None,
        });
        
        contract.sign("p1", "signature_data_1");
        assert_eq!(contract.signatures.len(), 1);
    }
}
