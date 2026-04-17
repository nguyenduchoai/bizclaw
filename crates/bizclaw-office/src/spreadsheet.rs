use crate::types::{Cell, CellStyle, CellValue, Sheet, Spreadsheet};
use anyhow::Result;
use chrono::Utc;

pub struct SpreadsheetBuilder {
    spreadsheet: Spreadsheet,
}

impl SpreadsheetBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            spreadsheet: Spreadsheet {
                id: uuid_v4(),
                name: name.into(),
                sheets: Vec::new(),
            },
        }
    }

    pub fn add_sheet(&mut self, name: impl Into<String>) {
        self.spreadsheet.sheets.push(Sheet {
            name: name.into(),
            headers: Vec::new(),
            rows: Vec::new(),
            column_widths: None,
        });
    }

    pub fn current_sheet(&mut self) -> Option<&mut Sheet> {
        self.spreadsheet.sheets.last_mut()
    }

    pub fn set_headers(&mut self, headers: Vec<String>) {
        if let Some(sheet) = self.current_sheet() {
            sheet.headers = headers;
        }
    }

    pub fn add_row(&mut self, values: Vec<CellValue>) {
        if let Some(sheet) = self.current_sheet() {
            let cells: Vec<Cell> = values
                .into_iter()
                .map(|v| Cell {
                    value: v,
                    style: None,
                })
                .collect();
            sheet.rows.push(cells);
        }
    }

    pub fn add_styled_row(&mut self, values: Vec<Cell>, header_row: bool) {
        if let Some(sheet) = self.current_sheet() {
            let styled_cells: Vec<Cell> = if header_row {
                values
                    .into_iter()
                    .map(|mut c| {
                        c.style = Some(CellStyle {
                            bold: true,
                            background_color: Some("#4CAF50".to_string()),
                            text_color: Some("#FFFFFF".to_string()),
                            ..Default::default()
                        });
                        c
                    })
                    .collect()
            } else {
                values
            };
            sheet.rows.push(styled_cells);
        }
    }

    pub fn add_inventory_sheet(&mut self, items: Vec<InventoryItem>) -> Result<()> {
        self.add_sheet("Tồn kho");
        self.set_headers(vec![
            "Mã SP".to_string(),
            "Tên sản phẩm".to_string(),
            "Danh mục".to_string(),
            "Tồn kho".to_string(),
            "Giá vốn".to_string(),
            "Giá bán".to_string(),
            "Trạng thái".to_string(),
        ]);

        for item in items {
            let status = if item.quantity < item.min_stock {
                "Sắp hết"
            } else if item.quantity > item.max_stock {
                "Quá nhiều"
            } else {
                "Bình thường"
            };

            self.add_row(vec![
                CellValue::Text(item.product_id),
                CellValue::Text(item.name),
                CellValue::Text(item.category),
                CellValue::Number(item.quantity as f64),
                CellValue::Currency(item.cost_price, "VND".to_string()),
                CellValue::Currency(item.sell_price, "VND".to_string()),
                CellValue::Text(status.to_string()),
            ]);
        }

        Ok(())
    }

    pub fn add_sales_sheet(&mut self, sales: Vec<SalesRecord>) -> Result<()> {
        self.add_sheet("Bán hàng");
        self.set_headers(vec![
            "Ngày".to_string(),
            "Mã đơn".to_string(),
            "Sản phẩm".to_string(),
            "Số lượng".to_string(),
            "Đơn giá".to_string(),
            "Thành tiền".to_string(),
            "Lợi nhuận".to_string(),
        ]);

        let mut total_revenue = 0.0;
        let mut total_profit = 0.0;

        for sale in sales {
            let revenue = sale.quantity as f64 * sale.unit_price;
            let profit = sale.quantity as f64 * (sale.unit_price - sale.cost);
            total_revenue += revenue;
            total_profit += profit;

            self.add_row(vec![
                CellValue::Date(sale.date),
                CellValue::Text(sale.order_id),
                CellValue::Text(sale.product_name),
                CellValue::Number(sale.quantity as f64),
                CellValue::Currency(sale.unit_price, "VND".to_string()),
                CellValue::Currency(revenue, "VND".to_string()),
                CellValue::Currency(profit, "VND".to_string()),
            ]);
        }

        self.add_row(vec![
            CellValue::Text("TỔNG CỘNG".to_string()),
            CellValue::Empty,
            CellValue::Empty,
            CellValue::Empty,
            CellValue::Empty,
            CellValue::Currency(total_revenue, "VND".to_string()),
            CellValue::Currency(total_profit, "VND".to_string()),
        ]);

        Ok(())
    }

    pub fn add_summary_sheet(&mut self, revenue: f64, orders: i64, costs: f64) -> Result<()> {
        self.add_sheet("Tổng kết");

        self.set_headers(vec![
            "Chỉ tiêu".to_string(),
            "Giá trị".to_string(),
            "Ghi chú".to_string(),
        ]);

        let profit = revenue - costs;
        let margin = if revenue > 0.0 {
            (profit / revenue) * 100.0
        } else {
            0.0
        };
        let avg_order_value = if orders > 0 {
            revenue / orders as f64
        } else {
            0.0
        };

        let rows = vec![
            (
                "Tổng doanh thu",
                CellValue::Currency(revenue, "VND".to_string()),
                "Từ tất cả đơn hàng",
            ),
            (
                "Tổng chi phí",
                CellValue::Currency(costs, "VND".to_string()),
                "Giá vốn sản phẩm",
            ),
            (
                "Lợi nhuận gộp",
                CellValue::Currency(profit, "VND".to_string()),
                "Sau khi trừ chi phí",
            ),
            ("Biên lợi nhuận", CellValue::Percentage(margin), "Tỷ lệ %"),
            (
                "Số đơn hàng",
                CellValue::Number(orders as f64),
                "Tổng số đơn",
            ),
            (
                "Giá trị TB/đơn",
                CellValue::Currency(avg_order_value, "VND".to_string()),
                "Trung bình mỗi đơn",
            ),
        ];

        for (label, value, note) in rows {
            self.add_row(vec![
                CellValue::Text(label.to_string()),
                value,
                CellValue::Text(note.to_string()),
            ]);
        }

        Ok(())
    }

    pub fn build(self) -> Spreadsheet {
        self.spreadsheet
    }
}

pub struct InventoryItem {
    pub product_id: String,
    pub name: String,
    pub category: String,
    pub quantity: i32,
    pub cost_price: f64,
    pub sell_price: f64,
    pub min_stock: i32,
    pub max_stock: i32,
}

pub struct SalesRecord {
    pub date: chrono::DateTime<Utc>,
    pub order_id: String,
    pub product_name: String,
    pub quantity: i32,
    pub unit_price: f64,
    pub cost: f64,
}

pub fn export_to_csv(sheet: &Sheet) -> String {
    let mut csv = String::new();

    if !sheet.headers.is_empty() {
        csv.push_str(&sheet.headers.join(","));
        csv.push('\n');
    }

    for row in &sheet.rows {
        let row_str: Vec<String> = row
            .iter()
            .map(|cell| cell_value_to_string(&cell.value))
            .collect();
        csv.push_str(&row_str.join(","));
        csv.push('\n');
    }

    csv
}

fn cell_value_to_string(value: &CellValue) -> String {
    match value {
        CellValue::Text(s) => format!("\"{}\"", s.replace('"', "\"\"")),
        CellValue::Number(n) => n.to_string(),
        CellValue::Currency(amount, _) => amount.to_string(),
        CellValue::Percentage(n) => format!("{}%", n),
        CellValue::Boolean(b) => b.to_string(),
        CellValue::Date(dt) => dt.format("%Y-%m-%d").to_string(),
        CellValue::Empty => String::new(),
    }
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
    fn test_spreadsheet_builder() {
        let mut builder = SpreadsheetBuilder::new("Test Report");
        builder.add_sheet("Sheet 1");
        builder.add_sheet("Sheet 2");
        let spreadsheet = builder.build();

        assert_eq!(spreadsheet.sheets.len(), 2);
    }

    #[test]
    fn test_csv_export() {
        let mut builder = SpreadsheetBuilder::new("Test");
        builder.add_sheet("Data");
        builder.set_headers(vec!["Name".to_string(), "Value".to_string()]);
        builder.add_row(vec![
            CellValue::Text("Test".to_string()),
            CellValue::Number(42.0),
        ]);

        let sheet = &builder.spreadsheet.sheets[0];
        let csv = export_to_csv(sheet);

        assert!(csv.contains("Name,Value"));
        assert!(csv.contains("\"Test\",42"));
    }
}
