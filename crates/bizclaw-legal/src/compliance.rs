use serde::{Deserialize, Serialize};

use crate::contracts::Contract;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceCheck {
    pub regulation: Regulation,
    pub status: ComplianceStatus,
    pub description: String,
    pub severity: Severity,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceStatus {
    Compliant,
    Warning,
    Violation,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Regulation {
    LawOnEnterprises2020,
    LawOnCommercial2020,
    LawOnTaxAdministration2019,
    LawOnIntellectualProperty,
    LaborCode2019,
    CybersecurityLaw2018,
    PersonalDataProtection,
    ConsumerProtection,
    ECommerce,
    AntiMoneyLaundering,
}

impl Regulation {
    pub fn code(&self) -> &'static str {
        match self {
            Regulation::LawOnEnterprises2020 => "Luật Doanh nghiệp 2020",
            Regulation::LawOnCommercial2020 => "Luật Thương mại 2020",
            Regulation::LawOnTaxAdministration2019 => "Luật Quản lý thuế 2019",
            Regulation::LawOnIntellectualProperty => "Luật Sở hữu trí tuệ",
            Regulation::LaborCode2019 => "Bộ luật Lao động 2019",
            Regulation::CybersecurityLaw2018 => "Luật An ninh mạng 2018",
            Regulation::PersonalDataProtection => "Nghị định Bảo vệ Dữ liệu Cá nhân",
            Regulation::ConsumerProtection => "Luật Bảo vệ Quyền lợi Người tiêu dùng",
            Regulation::ECommerce => "Nghị định Thương mại điện tử",
            Regulation::AntiMoneyLaundering => "Luật Phòng, chống Rửa tiền",
        }
    }

    pub fn key_requirements(&self) -> Vec<&'static str> {
        match self {
            Regulation::LawOnEnterprises2020 => vec![
                "Đăng ký kinh doanh hợp lệ",
                "Đủ điều kiện kinh doanh",
                "Báo cáo tài chính đúng hạn",
            ],
            Regulation::LawOnCommercial2020 => vec![
                "Hợp đồng thương mại hợp lệ",
                "Thanh toán đúng hạn",
                "Bảo mật thông tin thương mại",
            ],
            Regulation::LawOnTaxAdministration2019 => vec![
                "Kê khai thuế đúng hạn",
                "Hóa đơn hợp lệ",
                "Lưu trữ chứng từ 10 năm",
            ],
            Regulation::LaborCode2019 => vec![
                "Hợp đồng lao động bằng văn bản",
                "Bảo hiểm xã hội",
                "Thanh toán lương đúng hạn",
            ],
            Regulation::CybersecurityLaw2018 => vec![
                "Lưu trữ dữ liệu tại Việt Nam",
                "Bảo vệ thông tin cá nhân",
                "Báo cáo sự cố an ninh mạng",
            ],
            _ => vec!["Tuân thủ quy định chung"],
        }
    }
}

pub struct ComplianceChecker {
    regulations: Vec<Regulation>,
}

impl ComplianceChecker {
    pub fn new() -> Self {
        Self {
            regulations: vec![
                Regulation::LawOnEnterprises2020,
                Regulation::LawOnCommercial2020,
                Regulation::LawOnTaxAdministration2019,
                Regulation::LaborCode2019,
                Regulation::CybersecurityLaw2018,
            ],
        }
    }

    pub async fn check_contract(&mut self, contract: &Contract) -> anyhow::Result<Vec<ComplianceCheck>> {
        let mut checks = Vec::new();

        for regulation in &self.regulations {
            let check = self.check_against_regulation(contract, *regulation);
            checks.push(check);
        }

        Ok(checks)
    }

    fn check_against_regulation(&self, contract: &Contract, regulation: Regulation) -> ComplianceCheck {
        match regulation {
            Regulation::LawOnCommercial2020 => {
                let has_valid_terms = !contract.terms.is_empty();
                let has_parties = contract.parties.len() >= 2;
                
                ComplianceCheck {
                    regulation,
                    status: if has_valid_terms && has_parties {
                        ComplianceStatus::Compliant
                    } else {
                        ComplianceStatus::Warning
                    },
                    description: "Kiểm tra hợp đồng thương mại theo Luật Thương mại 2020".to_string(),
                    severity: if has_valid_terms && has_parties { Severity::Info } else { Severity::Medium },
                    recommendations: if !has_valid_terms {
                        vec!["Thêm các điều khoản bắt buộc".to_string()]
                    } else {
                        vec![]
                    },
                }
            }
            Regulation::LawOnTaxAdministration2019 => {
                ComplianceCheck {
                    regulation,
                    status: ComplianceStatus::Compliant,
                    description: "Hóa đơn và chứng từ phải được lưu trữ theo quy định".to_string(),
                    severity: Severity::Info,
                    recommendations: vec![
                        "Lưu trữ hóa đơn điện tử ít nhất 10 năm".to_string(),
                        "Xuất hóa đơn VAT đúng thời điểm".to_string(),
                    ],
                }
            }
            Regulation::LaborCode2019 => {
                if contract.contract_type == crate::contracts::ContractType::Employment {
                    let has_salary_term = contract.terms.iter().any(|t| 
                        matches!(t.category, crate::contracts::TermCategory::Payment)
                    );
                    
                    ComplianceCheck {
                        regulation,
                        status: if has_salary_term { ComplianceStatus::Compliant } else { ComplianceStatus::Warning },
                        description: "Hợp đồng lao động phải có mức lương rõ ràng".to_string(),
                        severity: if has_salary_term { Severity::Info } else { Severity::High },
                        recommendations: if !has_salary_term {
                            vec!["Thêm điều khoản về lương và phúc lợi".to_string()]
                        } else {
                            vec![]
                        },
                    }
                } else {
                    ComplianceCheck {
                        regulation,
                        status: ComplianceStatus::NotApplicable,
                        description: "Không áp dụng cho loại hợp đồng này".to_string(),
                        severity: Severity::Info,
                        recommendations: vec![],
                    }
                }
            }
            _ => ComplianceCheck {
                regulation,
                status: ComplianceStatus::Compliant,
                description: format!("Tuân thủ {}", regulation.code()),
                severity: Severity::Info,
                recommendations: vec![],
            },
        }
    }
}

impl Default for ComplianceChecker {
    fn default() -> Self {
        Self::new()
    }
}
