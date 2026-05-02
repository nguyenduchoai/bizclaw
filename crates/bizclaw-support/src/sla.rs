pub struct SlaConfig {
    pub first_response_hours: i64,
    pub resolution_hours: i64,
    pub business_hours_only: bool,
}

impl SlaConfig {
    pub fn retail() -> Self {
        Self {
            first_response_hours: 2,
            resolution_hours: 24,
            business_hours_only: false,
        }
    }

    pub fn enterprise() -> Self {
        Self {
            first_response_hours: 1,
            resolution_hours: 8,
            business_hours_only: true,
        }
    }
}
