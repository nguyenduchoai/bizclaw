use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialReport {
    pub report_type: ReportType,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub revenue: i64,
    pub expenses: i64,
    pub net_income: i64,
    pub total_assets: i64,
    pub total_liabilities: i64,
    pub equity: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportType {
    BalanceSheet,
    IncomeStatement,
    CashFlow,
    TrialBalance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceSheet {
    pub date: DateTime<Utc>,
    pub assets: i64,
    pub liabilities: i64,
    pub equity: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomeStatement {
    pub period: String,
    pub revenue: i64,
    pub cost_of_goods_sold: i64,
    pub gross_profit: i64,
    pub operating_expenses: i64,
    pub operating_income: i64,
    pub other_income: i64,
    pub other_expenses: i64,
    pub net_income: i64,
}

impl IncomeStatement {
    pub fn calculate(revenue: i64, cogs: i64, opex: i64) -> Self {
        let gross_profit = revenue - cogs;
        let operating_income = gross_profit - opex;
        Self {
            period: Utc::now().format("%Y-%m").to_string(),
            revenue,
            cost_of_goods_sold: cogs,
            gross_profit,
            operating_expenses: opex,
            operating_income,
            other_income: 0,
            other_expenses: 0,
            net_income: operating_income,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashFlow {
    pub period: String,
    pub operating: i64,
    pub investing: i64,
    pub financing: i64,
    pub net_cash: i64,
}
