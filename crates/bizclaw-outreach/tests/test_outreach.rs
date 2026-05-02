#[cfg(test)]
mod tests {
    use bizclaw_outreach::{OutreachAgent, Lead, LeadStatus};
    use serde_json::json;

    #[tokio::test]
    async fn test_create_campaign() {
        let agent = OutreachAgent::new();
        
        let leads = vec![
            Lead::new("Nguyễn Văn A"),
            Lead::new("Trần Thị B"),
        ];
        
        let campaign = agent.create_campaign("Test Campaign", leads).await;
        assert!(campaign.is_ok());
        
        let campaign = campaign.unwrap();
        assert_eq!(campaign.name, "Test Campaign");
        assert_eq!(campaign.leads.len(), 2);
    }

    #[tokio::test]
    async fn test_lead_scoring() {
        let mut lead = Lead::new("Test Lead");
        assert_eq!(lead.score, 0);
        
        lead = lead.with_email("test@example.com");
        assert_eq!(lead.score, 10);
        
        lead = lead.with_phone("0909123456");
        assert_eq!(lead.score, 25);
        
        lead = lead.with_zalo("zalo123");
        assert_eq!(lead.score, 45);
    }

    #[tokio::test]
    async fn test_process_zalo_message() {
        let agent = OutreachAgent::new();
        
        let message = bizclaw_outreach::OutreachMessage {
            intent: "send_zalo_message".to_string(),
            payload: json!({
                "phone": "0909123456",
                "message": "Chào bạn!"
            }),
        };
        
        let response = agent.process(message).await.unwrap();
        assert!(response.success);
        assert_eq!(response.data["channel"], "zalo");
    }

    #[tokio::test]
    async fn test_add_lead_to_campaign() {
        let agent = OutreachAgent::new();
        
        let leads = vec![Lead::new("Existing Lead")];
        let campaign = agent.create_campaign("Test", leads).await.unwrap();
        
        let new_lead = Lead::new("New Lead").with_email("new@example.com");
        let result = agent.add_lead(&campaign.id, new_lead).await;
        assert!(result.is_ok());
    }
}
