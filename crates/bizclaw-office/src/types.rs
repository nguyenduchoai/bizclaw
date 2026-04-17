use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub content: Vec<DocumentElement>,
    pub metadata: DocumentMetadata,
    pub format: DocumentFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentFormat {
    Docx,
    Pdf,
    Markdown,
    Html,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub author: Option<String>,
    pub company: Option<String>,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub tags: Vec<String>,
}

impl Default for DocumentMetadata {
    fn default() -> Self {
        Self {
            author: None,
            company: None,
            created_at: Utc::now(),
            modified_at: Utc::now(),
            tags: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DocumentElement {
    Heading {
        level: u8,
        text: String,
    },
    Paragraph {
        text: String,
        style: Option<ParagraphStyle>,
    },
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
        style: Option<TableStyle>,
    },
    List {
        items: Vec<String>,
        ordered: bool,
    },
    Image {
        url: String,
        caption: Option<String>,
        width: Option<u32>,
        height: Option<u32>,
    },
    Code {
        language: String,
        code: String,
    },
    PageBreak,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParagraphStyle {
    pub bold: bool,
    pub italic: bool,
    pub font_size: Option<u16>,
    pub color: Option<String>,
    pub alignment: Option<TextAlignment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableStyle {
    pub header_bg: Option<String>,
    pub border: Option<bool>,
    pub striped: Option<bool>,
}

impl Default for ParagraphStyle {
    fn default() -> Self {
        Self {
            bold: false,
            italic: false,
            font_size: None,
            color: None,
            alignment: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub id: String,
    pub title: String,
    pub period: ReportPeriod,
    pub sections: Vec<ReportSection>,
    pub summary: ReportSummary,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportPeriod {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSection {
    pub title: String,
    pub content: Vec<DocumentElement>,
    pub charts: Vec<Chart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    pub total_revenue: f64,
    pub total_orders: i64,
    pub average_order_value: f64,
    pub top_products: Vec<TopProduct>,
    pub highlights: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopProduct {
    pub name: String,
    pub quantity: i64,
    pub revenue: f64,
    pub rank: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chart {
    pub chart_type: ChartType,
    pub title: String,
    pub data: ChartData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChartType {
    Bar,
    Line,
    Pie,
    Column,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartData {
    pub labels: Vec<String>,
    pub datasets: Vec<Dataset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dataset {
    pub label: String,
    pub values: Vec<f64>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spreadsheet {
    pub id: String,
    pub name: String,
    pub sheets: Vec<Sheet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sheet {
    pub name: String,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<Cell>>,
    pub column_widths: Option<Vec<u32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cell {
    pub value: CellValue,
    pub style: Option<CellStyle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CellValue {
    Text(String),
    Number(f64),
    Currency(f64, String),
    Percentage(f64),
    Boolean(bool),
    Date(DateTime<Utc>),
    Empty,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellStyle {
    pub bold: bool,
    pub background_color: Option<String>,
    pub text_color: Option<String>,
    pub number_format: Option<String>,
    pub alignment: Option<TextAlignment>,
}

impl Default for CellStyle {
    fn default() -> Self {
        Self {
            bold: false,
            background_color: None,
            text_color: None,
            number_format: None,
            alignment: None,
        }
    }
}

impl Cell {
    pub fn text(s: impl Into<String>) -> Self {
        Self {
            value: CellValue::Text(s.into()),
            style: None,
        }
    }

    pub fn number(n: f64) -> Self {
        Self {
            value: CellValue::Number(n),
            style: None,
        }
    }

    pub fn currency(amount: f64, currency: &str) -> Self {
        Self {
            value: CellValue::Currency(amount, currency.to_string()),
            style: None,
        }
    }

    pub fn percentage(n: f64) -> Self {
        Self {
            value: CellValue::Percentage(n),
            style: None,
        }
    }

    pub fn date(dt: DateTime<Utc>) -> Self {
        Self {
            value: CellValue::Date(dt),
            style: None,
        }
    }
}
