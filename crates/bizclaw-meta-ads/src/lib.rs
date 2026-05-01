pub mod types;
pub mod campaigns;
pub mod adsets;
pub mod ads;
pub mod insights;
pub mod audiences;

pub use types::*;
pub use campaigns::CampaignManager;
pub use adsets::AdsetManager;
pub use ads::AdManager;
pub use insights::InsightsReporter;
pub use audiences::AudienceManager;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_calculation() {
        let daily_budget = 100_000.0_f64;
        let days = 30.0;
        let lifetime = calculate_lifetime_budget(daily_budget, days);
        assert!((lifetime - 3_000_000.0).abs() < 0.01);
    }

    #[test]
    fn test_objective_conversion() {
        assert_eq!(CampaignObjective::from_string("CONVERSIONS"), CampaignObjective::Conversions);
        assert_eq!(CampaignObjective::from_string("REACH"), CampaignObjective::Reach);
        assert_eq!(CampaignObjective::from_string("TRAFFIC"), CampaignObjective::Traffic);
    }

    #[test]
    fn test_status_conversion() {
        assert_eq!(CampaignStatus::from_string("PAUSED"), CampaignStatus::Paused);
        assert_eq!(CampaignStatus::from_string("ACTIVE"), CampaignStatus::Active);
    }
}
