#[cfg(test)]
mod tests {
    use bizclaw_analytics::{AnalyticsAgent, Dashboard, KPI, KPICategory};

    #[tokio::test]
    async fn test_create_dashboard() {
        let agent = AnalyticsAgent::new();
        
        let result = agent.create_dashboard("Test Dashboard").await;
        assert!(result.is_ok());
        
        let dashboard = result.unwrap();
        assert_eq!(dashboard.name, "Test Dashboard");
    }

    #[tokio::test]
    async fn test_track_kpi() {
        let agent = AnalyticsAgent::new();
        
        let kpi = KPI::new("Revenue", KPICategory::Revenue, 100_000_000.0)
            .with_unit("VND");
        
        let result = agent.track_kpi(kpi).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_generate_report() {
        let agent = AnalyticsAgent::new();
        
        let now = chrono::Utc::now();
        let result = agent.generate_report(
            bizclaw_analytics::ReportType::MonthlySummary,
            now,
            now,
        ).await;
        
        assert!(result.is_ok());
        
        let report = result.unwrap();
        assert_eq!(report.report_type, bizclaw_analytics::ReportType::MonthlySummary);
    }

    #[tokio::test]
    async fn test_get_insights() {
        let agent = AnalyticsAgent::new();
        
        let result = agent.get_insights("30d").await;
        assert!(result.is_ok());
        
        let insights = result.unwrap();
        assert!(!insights.is_empty());
    }

    #[test]
    fn test_dashboard_creation() {
        let dashboard = Dashboard::new("Test");
        
        assert!(!dashboard.id.is_empty());
        assert_eq!(dashboard.name, "Test");
    }

    #[test]
    fn test_kpi_creation() {
        let kpi = KPI::new("Revenue", KPICategory::Revenue, 100_000_000.0);
        
        assert!(!kpi.id.is_empty());
        assert_eq!(kpi.name, "Revenue");
        assert_eq!(kpi.status, bizclaw_analytics::KPIStatus::NotStarted);
    }

    #[test]
    fn test_kpi_update() {
        let mut kpi = KPI::new("Test", KPICategory::Sales, 100.0);
        
        kpi.update_value(50.0);
        assert_eq!(kpi.current_value, 50.0);
        assert_eq!(kpi.history.len(), 1);
    }

    #[test]
    fn test_report_creation() {
        let report = bizclaw_analytics::Report::new(
            bizclaw_analytics::ReportType::Sales, 
            "Sales Report"
        );
        
        assert!(!report.id.is_empty());
        assert_eq!(report.report_type, bizclaw_analytics::ReportType::Sales);
    }
}
