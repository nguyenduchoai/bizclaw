use serde::{Deserialize, Serialize};
use crate::contracts::{Contract, ContractType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractTemplate {
    pub id: String,
    pub name: String,
    pub contract_type: ContractType,
    pub description: String,
    pub content: String,
    pub required_fields: Vec<String>,
    pub optional_fields: Vec<String>,
}

pub struct TemplateLibrary {
    templates: std::collections::HashMap<String, ContractTemplate>,
}

impl TemplateLibrary {
    pub fn new() -> Self {
        let mut library = Self {
            templates: std::collections::HashMap::new(),
        };
        library.load_vietnamese_templates();
        library
    }

    fn load_vietnamese_templates(&mut self) {
        self.templates.insert(
            "service_agreement_vn".to_string(),
            ContractTemplate {
                id: "service_agreement_vn".to_string(),
                name: "Hợp đồng Dịch vụ (Việt Nam)".to_string(),
                contract_type: ContractType::Service,
                description: "Hợp đồng cung cấp dịch vụ theo pháp luật Việt Nam".to_string(),
                content: r#"CỘNG HÒA XÃ HỘI CHỦ NGHĨA VIỆT NAM
Độc lập - Tự do - Hạnh phúc

HỢP ĐỒNG DỊCH VỤ
Số: {{contract_number}}

CỘNG HÒA XÃ HỘI CHỦ NGHĨA VIỆT NAM
Độc lập - Tự do - Hạnh phúc

HỢP ĐỒNG DỊCH VỤ
Số: {{contract_number}}

Các bên ký hợp đồng:
BÊN A: {{party_a_name}}
Địa chỉ: {{party_a_address}}
Đại diện: {{party_a_representative}}
MST: {{party_a_tax_code}}

BÊN B: {{party_b_name}}
Địa chỉ: {{party_b_address}}
Đại diện: {{party_b_representative}}
MST: {{party_b_tax_code}}

 Sau khi thỏa thuận, hai bên thống nhất ký hợp đồng với các điều khoản sau:

ĐIỀU 1: NỘI DUNG DỊCH VỤ
{{service_description}}

ĐIỀU 2: GIÁ TRỊ VÀ THANH TOÁN
2.1. Giá trị hợp đồng: {{amount}} {{currency}}
2.2. Phương thức thanh toán: {{payment_method}}
2.3. Thời hạn thanh toán: {{payment_term}}

ĐIỀU 3: THỜI HẠN HỢP ĐỒNG
3.1. Hợp đồng có hiệu lực từ ngày {{start_date}}
3.2. Hợp đồng hết hiệu lực ngày {{end_date}}

ĐIỀU 4: QUYỀN VÀ NGHĨA VỤ CÁC BÊN
{{obligations}}

ĐIỀU 5: GIẢI QUYẾT TRANH CHẤP
Các bên thống nhất giải quyết tranh chấp thông qua thương lượng. Nếu không thể thương lượng, tranh chấp sẽ được giải quyết tại Tòa án nhân dân có thẩm quyền.

ĐIỀU 6: ĐIỀU KHOẢN CHUNG
{{general_terms}}

Hợp đồng này được lập thành {{copy_count}} bản có giá trị pháp lý như nhau, mỗi bên giữ {{copy_each}} bản.

ĐẠI DIỆN BÊN A                    ĐẠI DIỆN BÊN B"#.to_string(),
                required_fields: vec![
                    "party_a_name".to_string(),
                    "party_b_name".to_string(),
                    "service_description".to_string(),
                    "amount".to_string(),
                    "start_date".to_string(),
                ],
                optional_fields: vec![
                    "party_a_address".to_string(),
                    "party_b_address".to_string(),
                    "end_date".to_string(),
                    "payment_method".to_string(),
                ],
            },
        );

        self.templates.insert(
            "nda_vn".to_string(),
            ContractTemplate {
                id: "nda_vn".to_string(),
                name: "Thỏa thuận Bảo mật (NDA)".to_string(),
                contract_type: ContractType::NDA,
                description: "Thỏa thuận bảo mật thông tin theo pháp luật Việt Nam".to_string(),
                content: r#"THỎA THUẬN BẢO MẬT

Ngày {{date}}, tại {{location}}

BÊN TIẾT LỘ: {{disclosing_party}}
BÊN NHẬN: {{receiving_party}}

1. THÔNG TIN BẢO MẬT
{{confidential_info}}

2. NGHĨA VỤ BÊN NHẬN
- Không tiết lộ thông tin cho bên thứ ba
- Chỉ sử dụng cho mục đích {{purpose}}
- Lưu trữ an toàn và bảo mật

3. THỜI HẠN
Thỏa thuận này có hiệu lực trong {{duration}} năm kể từ ngày ký.

4. CHẾ TÀI
Vi phạm thỏa thuận sẽ chịu trách nhiệm bồi thường {{compensation}}."#.to_string(),
                required_fields: vec![
                    "disclosing_party".to_string(),
                    "receiving_party".to_string(),
                    "purpose".to_string(),
                ],
                optional_fields: vec![
                    "date".to_string(),
                    "location".to_string(),
                    "duration".to_string(),
                ],
            },
        );

        self.templates.insert(
            "employment_contract_vn".to_string(),
            ContractTemplate {
                id: "employment_contract_vn".to_string(),
                name: "Hợp đồng Lao động".to_string(),
                contract_type: ContractType::Employment,
                description: "Hợp đồng lao động theo Bộ luật Lao động 2019".to_string(),
                content: r#"HỢP ĐỒNG LAO ĐỘNG
Số: {{contract_number}}

Căn cứ Bộ luật Lao động 2019;

Các bên thống nhất ký hợp đồng lao động với các điều khoản sau:

ĐIỀU 1: BÊN A (NGƯỜI SỬ DỤNG LAO ĐỘNG)
Tên công ty: {{employer_name}}
Địa chỉ: {{employer_address}}
MST: {{employer_tax_code}}
Đại diện: {{employer_representative}}
Chức vụ: {{employer_position}}

ĐIỀU 2: BÊN B (NGƯỜI LAO ĐỘNG)
Họ và tên: {{employee_name}}
Ngày sinh: {{employee_dob}}
Địa chỉ: {{employee_address}}
Số CCCD: {{employee_id}}
Ngày cấp: {{employee_id_date}}
Nơi cấp: {{employee_id_place}}

ĐIỀU 3: NỘI DUNG CÔNG VIỆC
3.1. Vị trí công việc: {{job_title}}
3.2. Phòng ban: {{department}}
3.3. Địa điểm làm việc: {{work_location}}

ĐIỀU 4: THỜI HẠN HỢP ĐỒNG
{{contract_duration}}

ĐIỀU 5: LƯƠNG VÀ PHÚC LỢI
5.1. Mức lương: {{salary}} {{currency}}/tháng
5.2. Hình thức trả lương: {{payment_frequency}}
5.3. Ngày trả lương: {{payment_date}}
5.4. Phụ cấp: {{allowances}}

ĐIỀU 6: BẢO HIỂM
Theo quy định của pháp luật về bảo hiểm xã hội, y tế, thất nghiệp."#.to_string(),
                required_fields: vec![
                    "employer_name".to_string(),
                    "employee_name".to_string(),
                    "job_title".to_string(),
                    "salary".to_string(),
                ],
                optional_fields: vec![
                    "department".to_string(),
                    "allowances".to_string(),
                    "contract_duration".to_string(),
                ],
            },
        );
    }

    pub fn get(&self, id: &str) -> anyhow::Result<&ContractTemplate> {
        self.templates
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("Template not found: {}", id))
    }

    pub fn list_templates(&self) -> Vec<&ContractTemplate> {
        self.templates.values().collect()
    }

    pub fn fill_template(
        &self,
        template_id: &str,
        _parties: &str,
        fields: serde_json::Value,
    ) -> anyhow::Result<Contract> {
        let template = self.get(template_id)?;
        
        let mut contract = Contract::new(template.contract_type, &template.name);
        
        if let Some(obj) = fields.as_object() {
            if let Some(number) = obj.get("contract_number").and_then(|v| v.as_str()) {
                contract.number = number.to_string();
            }
            if let Some(amount) = obj.get("amount").and_then(|v| v.as_i64()) {
                contract.value = Some(amount);
            }
        }
        
        Ok(contract)
    }
}

impl Default for TemplateLibrary {
    fn default() -> Self {
        Self::new()
    }
}
