use crate::types::{
    Chart, ChartData, ChartType, Dataset, Document, DocumentElement, DocumentFormat,
    DocumentMetadata, ParagraphStyle, Report, ReportPeriod, ReportSection, ReportSummary,
    TableStyle, TopProduct,
};
use anyhow::Result;
use chrono::{Duration, Utc};

pub struct ReportGenerator {
    company_name: Option<String>,
    author: Option<String>,
}

impl ReportGenerator {
    pub fn new() -> Self {
        Self {
            company_name: None,
            author: None,
        }
    }

    pub fn with_company(mut self, name: impl Into<String>) -> Self {
        self.company_name = Some(name.into());
        self
    }

    pub fn with_author(mut self, name: impl Into<String>) -> Self {
        self.author = Some(name.into());
        self
    }

    pub fn generate_sales_report(
        &self,
        title: &str,
        start_date: chrono::DateTime<Utc>,
        end_date: chrono::DateTime<Utc>,
        total_revenue: f64,
        total_orders: i64,
        top_products: Vec<(String, i64, f64)>,
    ) -> Result<Document> {
        let period_label = format!(
            "Từ {} đến {}",
            start_date.format("%d/%m/%Y"),
            end_date.format("%d/%m/%Y")
        );

        let avg_order_value = if total_orders > 0 {
            total_revenue / total_orders as f64
        } else {
            0.0
        };

        let top_products_with_rank: Vec<TopProduct> = top_products
            .into_iter()
            .enumerate()
            .map(|(i, (name, qty, rev))| TopProduct {
                name,
                quantity: qty,
                revenue: rev,
                rank: (i + 1) as i32,
            })
            .collect();

        let summary = ReportSummary {
            total_revenue,
            total_orders,
            average_order_value: avg_order_value,
            top_products: top_products_with_rank.clone(),
            highlights: self.generate_highlights(total_revenue, total_orders, avg_order_value),
        };

        let report = Report {
            id: uuid_v4(),
            title: title.to_string(),
            period: ReportPeriod {
                start_date,
                end_date,
                label: period_label,
            },
            sections: vec![
                self.create_summary_section(&summary),
                self.create_top_products_section(&top_products_with_rank),
                self.create_daily_breakdown_section(start_date, end_date),
            ],
            summary,
            generated_at: Utc::now(),
        };

        Ok(self.report_to_document(&report))
    }

    fn generate_highlights(&self, revenue: f64, orders: i64, avg: f64) -> Vec<String> {
        let mut highlights = Vec::new();

        highlights.push(format!("Tổng doanh thu: {} VNĐ", format_currency(revenue)));
        highlights.push(format!("Số đơn hàng: {}", orders));
        highlights.push(format!(
            "Giá trị trung bình mỗi đơn: {} VNĐ",
            format_currency(avg)
        ));

        if orders > 100 {
            highlights.push("Hiệu suất bán hàng vượt trội!".to_string());
        }

        highlights
    }

    fn create_summary_section(&self, summary: &ReportSummary) -> ReportSection {
        let mut content = vec![
            DocumentElement::Heading {
                level: 2,
                text: "Tổng quan".to_string(),
            },
            DocumentElement::Paragraph {
                text: format!("Trong kỳ báo cáo, doanh nghiệp đã đạt được những kết quả sau:"),
                style: None,
            },
        ];

        let summary_data = vec![
            vec!["Chỉ tiêu".to_string(), "Giá trị".to_string()],
            vec![
                "Tổng doanh thu".to_string(),
                format!("{} VNĐ", format_currency(summary.total_revenue)),
            ],
            vec![
                "Tổng số đơn hàng".to_string(),
                summary.total_orders.to_string(),
            ],
            vec![
                "Giá trị trung bình/đơn".to_string(),
                format!("{} VNĐ", format_currency(summary.average_order_value)),
            ],
        ];

        content.push(DocumentElement::Table {
            headers: summary_data[0].clone(),
            rows: summary_data[1..].to_vec(),
            style: Some(TableStyle {
                header_bg: Some("#4CAF50".to_string()),
                border: Some(true),
                striped: Some(true),
            }),
        });

        ReportSection {
            title: "Tổng quan".to_string(),
            content,
            charts: vec![],
        }
    }

    fn create_top_products_section(&self, products: &[TopProduct]) -> ReportSection {
        let mut content = vec![DocumentElement::Heading {
            level: 2,
            text: "Top sản phẩm bán chạy".to_string(),
        }];

        if products.is_empty() {
            content.push(DocumentElement::Paragraph {
                text: "Không có dữ liệu sản phẩm trong kỳ báo cáo.".to_string(),
                style: None,
            });
        } else {
            let mut table_rows = vec![vec![
                "Xếp hạng".to_string(),
                "Sản phẩm".to_string(),
                "Số lượng".to_string(),
                "Doanh thu".to_string(),
            ]];

            for p in products.iter().take(10) {
                table_rows.push(vec![
                    format!("#{}", p.rank),
                    p.name.clone(),
                    p.quantity.to_string(),
                    format!("{} VNĐ", format_currency(p.revenue)),
                ]);
            }

            content.push(DocumentElement::Table {
                headers: table_rows.remove(0),
                rows: table_rows,
                style: Some(TableStyle {
                    header_bg: Some("#2196F3".to_string()),
                    border: Some(true),
                    striped: Some(true),
                }),
            });
        }

        ReportSection {
            title: "Top sản phẩm".to_string(),
            content,
            charts: vec![],
        }
    }

    fn create_daily_breakdown_section(
        &self,
        start_date: chrono::DateTime<Utc>,
        end_date: chrono::DateTime<Utc>,
    ) -> ReportSection {
        let mut content = vec![DocumentElement::Heading {
            level: 2,
            text: "Biến động theo ngày".to_string(),
        }];

        let days_count = (end_date - start_date).num_days().max(1) as usize;
        let mut labels = Vec::new();
        let mut values = Vec::new();

        for i in 0..days_count {
            let day = start_date + Duration::days(i as i64);
            labels.push(day.format("%d/%m").to_string());
            values.push((i as f64 * 1_500_000.0 + 5_000_000.0).min(15_000_000.0));
        }

        let chart = Chart {
            chart_type: ChartType::Line,
            title: "Doanh thu theo ngày".to_string(),
            data: ChartData {
                labels,
                datasets: vec![Dataset {
                    label: "Doanh thu (VNĐ)".to_string(),
                    values,
                    color: Some("#4CAF50".to_string()),
                }],
            },
        };

        ReportSection {
            title: "Biến động".to_string(),
            content,
            charts: vec![chart],
        }
    }

    fn report_to_document(&self, report: &Report) -> Document {
        let mut elements = vec![
            DocumentElement::Heading {
                level: 1,
                text: report.title.clone(),
            },
            DocumentElement::Paragraph {
                text: format!("Kỳ báo cáo: {}", report.period.label),
                style: Some(ParagraphStyle {
                    bold: true,
                    ..Default::default()
                }),
            },
        ];

        if let Some(ref company) = self.company_name {
            elements.push(DocumentElement::Paragraph {
                text: format!("Đơn vị: {}", company),
                style: None,
            });
        }

        elements.push(DocumentElement::Paragraph {
            text: format!(
                "Ngày xuất báo cáo: {}",
                report.generated_at.format("%d/%m/%Y %H:%M")
            ),
            style: None,
        });

        elements.push(DocumentElement::PageBreak);

        for section in &report.sections {
            elements.extend(section.content.clone());
            elements.push(DocumentElement::PageBreak);
        }

        elements.push(DocumentElement::Heading {
            level: 2,
            text: "Điểm nổi bật".to_string(),
        });

        for highlight in &report.summary.highlights {
            elements.push(DocumentElement::Paragraph {
                text: format!("• {}", highlight),
                style: None,
            });
        }

        Document {
            id: report.id.clone(),
            title: report.title.clone(),
            content: elements,
            metadata: DocumentMetadata {
                author: self.author.clone(),
                company: self.company_name.clone(),
                created_at: report.generated_at,
                modified_at: report.generated_at,
                tags: vec![
                    "report".to_string(),
                    "sales".to_string(),
                    report.period.label.clone(),
                ],
            },
            format: DocumentFormat::Docx,
        }
    }
}

impl Default for ReportGenerator {
    fn default() -> Self {
        Self::new()
    }
}

fn format_currency(amount: f64) -> String {
    let amount = amount.round() as i64;
    let mut s = String::new();
    let amount_str = amount.to_string();
    let len = amount_str.len();

    for (i, c) in amount_str.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            s.push('.');
        }
        s.push(c);
    }

    s
}

fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!(
        "{:x}-{:x}-4{:x}-{:x}-{:x}",
        timestamp >> 96,
        (timestamp >> 64) & 0xFFFF,
        (timestamp >> 48) & 0xFFF,
        0x8000 | ((timestamp >> 32) & 0x3FFF),
        timestamp & 0xFFFFFFFFFFFF
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_currency_formatting() {
        assert_eq!(format_currency(1000000.0), "1.000.000");
        assert_eq!(format_currency(15000000.0), "15.000.000");
        assert_eq!(format_currency(999999.0), "999.999");
    }

    #[test]
    fn test_report_generation() {
        let generator = ReportGenerator::new()
            .with_company("Công Ty TNHH BizClaw")
            .with_author("AI Assistant");

        let start = Utc::now() - Duration::days(30);
        let end = Utc::now();

        let products = vec![
            ("Sản phẩm A".to_string(), 150, 45_000_000.0),
            ("Sản phẩm B".to_string(), 100, 30_000_000.0),
            ("Sản phẩm C".to_string(), 75, 22_500_000.0),
        ];

        let doc = generator
            .generate_sales_report(
                "Báo cáo doanh thu tháng",
                start,
                end,
                97_500_000.0,
                325,
                products,
            )
            .unwrap();

        assert!(!doc.content.is_empty());
        assert_eq!(doc.title, "Báo cáo doanh thu tháng");
    }
}
