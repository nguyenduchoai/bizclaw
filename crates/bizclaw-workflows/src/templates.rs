//! Pre-built workflow templates — ready to use out of the box.

use crate::step::{CollectStrategy, Condition, LoopConfig, StepType, Workflow, WorkflowStep};

/// Get all built-in workflow templates.
pub fn builtin_workflows() -> Vec<Workflow> {
    vec![
        content_pipeline(),
        expert_consensus(),
        quality_pipeline(),
        research_pipeline(),
        translation_pipeline(),
        code_review_pipeline(),
        slide_creator(),
        // CEO Workflows
        meeting_recap(),
        ceo_daily_briefing(),
        competitor_analysis(),
        proposal_generator(),
        weekly_report(),
        // Business Operations
        email_drip_campaign(),
        hiring_pipeline(),
        customer_feedback_analysis(),
        contract_review(),
        product_launch_checklist(),
        // Agent Team — Micro SaaS Operations
        vigor_trend_scout(),
        vigor_blog_pipeline(),
        fidus_health_check(),
        fidus_cost_tracker(),
        optimo_funnel_audit(),
        mercury_outreach(),
        // Automation
        auto_trend_social_pipeline(),
    ]
}

/// Content creation pipeline: Draft → Review → Edit → Publish.
pub fn content_pipeline() -> Workflow {
    Workflow::new("content_pipeline", "Content creation pipeline — Draft → Review → Edit → Publish")
        .with_tags(vec!["content", "writing", "marketing"])
        .add_step(
            WorkflowStep::new("draft", "content-writer", StepType::Sequential)
                .with_prompt("Write a comprehensive article about: {{input}}")
                .with_timeout(600)
                .with_retries(1),
        )
        .add_step(
            WorkflowStep::new("review", "content-reviewer", StepType::Sequential)
                .with_prompt("Review this article for quality, accuracy, and engagement. Provide specific feedback and suggested improvements:\n\n{{input}}")
                .with_timeout(300),
        )
        .add_step(
            WorkflowStep::new("edit", "content-editor", StepType::Sequential)
                .with_prompt("Apply the review feedback and create the final polished version of this article:\n\n{{input}}")
                .with_timeout(300),
        )
}

/// Auto Trend to Zalo & Facebook Pipeline: Scan Trends -> Write Content -> Post Zalo & Facebook.
pub fn auto_trend_social_pipeline() -> Workflow {
    Workflow::new(
        "auto_trend_social",
        "Tự động quét Trend -> Viết bài -> Đăng Zalo & Facebook",
    )
    .with_tags(vec!["trend", "zalo", "facebook", "automation", "mama"])
    .add_step(
        WorkflowStep::new("scan_trends", "trend-scout", StepType::Sequential)
            .with_prompt("Dùng bộ TrendRadar để quét các tin tức nóng nhất hôm nay về chủ đề: {{input}}. Phân tích và tổng hợp lại thành top 3 tin nổi bật nhất (tóm tắt ý chính).")
            .with_timeout(300),
    )
    .add_step(
        WorkflowStep::new("write_content", "content-writer", StepType::Sequential)
            .with_prompt("Dựa vào 3 tin tức nóng nhất sau đây, hãy viết một bài đăng (post) thật thu hút, ngôn từ sắc bén, sử dụng emoji phù hợp và có call-to-action.\nChỉ trả về nội dung bài viết, không thêm lời giải thích:\n\n{{input}}")
            .with_timeout(300),
    )
    .add_step(
        WorkflowStep::new("post_social", "mama", StepType::Sequential)
            .with_prompt("Bạn là Mama Agent. Nhận nội dung bài viết sau và thực hiện 2 việc ĐỒNG THỜI:\n1. Gọi action `zalo_post` để đăng lên Nhật ký Zalo.\n2. Gọi action `facebook_post` để đăng lên Facebook của sếp.\n\nNội dung bài viết:\n{{input}}")
            .with_timeout(120),
    )
}

/// Expert consensus: 3 experts analyze independently → vote/merge.
pub fn expert_consensus() -> Workflow {
    Workflow::new(
        "expert_consensus",
        "Expert consensus — 3 independent analyses merged into one",
    )
    .with_tags(vec!["analysis", "consensus", "decision"])
    .add_step(
        WorkflowStep::new("expert-a", "analyst-a", StepType::Sequential)
            .with_prompt("As Expert A, analyze this independently:\n\n{{input}}"),
    )
    .add_step(
        WorkflowStep::new("expert-b", "analyst-b", StepType::Sequential)
            .with_prompt("As Expert B, analyze this independently:\n\n{{input}}"),
    )
    .add_step(
        WorkflowStep::new("expert-c", "analyst-c", StepType::Sequential)
            .with_prompt("As Expert C, analyze this independently:\n\n{{input}}"),
    )
    .add_step(WorkflowStep::new(
        "parallel-analysis",
        "coordinator",
        StepType::FanOut {
            parallel_steps: vec!["expert-a".into(), "expert-b".into(), "expert-c".into()],
        },
    ))
    .add_step(WorkflowStep::new(
        "merge",
        "coordinator",
        StepType::Collect {
            strategy: CollectStrategy::Merge,
            evaluator: None,
        },
    ))
}

/// Quality pipeline with evaluate loop: generate → review → revise until approved.
pub fn quality_pipeline() -> Workflow {
    Workflow::new(
        "quality_pipeline",
        "Quality-gated pipeline — generate and revise until approved",
    )
    .with_tags(vec!["quality", "review", "iterate"])
    .add_step(
        WorkflowStep::new("generate", "writer", StepType::Sequential)
            .with_prompt("Create high-quality content for: {{input}}")
            .with_timeout(600),
    )
    .add_step(WorkflowStep::new(
        "refine",
        "reviewer",
        StepType::Loop {
            body_step: "generate".into(),
            config: LoopConfig::new(3, Condition::new("quality", "contains", "APPROVED")),
        },
    ))
}

/// Research pipeline: Search → Analyze → Synthesize → Report.
pub fn research_pipeline() -> Workflow {
    Workflow::new("research_pipeline", "Research pipeline — Search → Analyze → Synthesize → Report")
        .with_tags(vec!["research", "analysis", "report"])
        .add_step(
            WorkflowStep::new("search", "researcher", StepType::Sequential)
                .with_prompt("Research the following topic thoroughly. Find key facts, data, and sources:\n\n{{input}}")
                .with_timeout(600),
        )
        .add_step(
            WorkflowStep::new("analyze", "analyst", StepType::Sequential)
                .with_prompt("Analyze the research findings below. Identify patterns, insights, and key takeaways:\n\n{{input}}")
                .with_timeout(300),
        )
        .add_step(
            WorkflowStep::new("synthesize", "synthesizer", StepType::Sequential)
                .with_prompt("Synthesize the analysis into a coherent narrative with conclusions and recommendations:\n\n{{input}}")
                .with_timeout(300),
        )
        .add_step(
            WorkflowStep::new("report", "report-writer", StepType::Sequential)
                .with_prompt("Format the synthesis into a professional report with executive summary, findings, and next steps:\n\n{{input}}")
                .with_timeout(300),
        )
}

/// Translation pipeline with quality check.
pub fn translation_pipeline() -> Workflow {
    Workflow::new("translation_pipeline", "Translation with quality verification")
        .with_tags(vec!["translation", "i18n", "quality"])
        .add_step(
            WorkflowStep::new("translate", "translator", StepType::Sequential)
                .with_prompt("Translate the following text to the target language, maintaining tone and meaning:\n\n{{input}}")
                .with_retries(1),
        )
        .add_step(
            WorkflowStep::new("verify", "translation-reviewer", StepType::Sequential)
                .with_prompt("Review this translation for accuracy, naturalness, and cultural appropriateness. If issues found, provide the corrected version:\n\n{{input}}")
                .optional(),
        )
}

/// Code review pipeline: Analyze → Security check → Style check → Summary.
pub fn code_review_pipeline() -> Workflow {
    Workflow::new("code_review", "Code review pipeline — analyze, security, style, summary")
        .with_tags(vec!["code", "review", "security"])
        .add_step(
            WorkflowStep::new("analyze", "code-analyst", StepType::Sequential)
                .with_prompt("Analyze this code for bugs, logic errors, and potential improvements:\n\n{{input}}")
                .with_timeout(600),
        )
        .add_step(
            WorkflowStep::new("security", "security-expert", StepType::Sequential)
                .with_prompt("Review this code for security vulnerabilities (injection, auth bypass, data exposure):\n\n{{input}}")
                .with_timeout(300),
        )
        .add_step(
            WorkflowStep::new("style", "style-checker", StepType::Sequential)
                .with_prompt("Review this code for style, readability, and best practices:\n\n{{input}}")
                .optional(),
        )
        .add_step(
            WorkflowStep::new(
                "summary",
                "coordinator",
                StepType::Transform {
                    template: "## Code Review Summary\n\n{{input}}".to_string(),
                },
            ),
        )
}
/// AI Slide Creator: Research → Plan → Generate (parallel) → Review → Export.
///
/// 4-stage pipeline implementing the full slide creation flow:
/// ① Research: Gather information, save to memory
/// ② Plan: LLM reasoning → structured slide outline with dependencies  
/// ③ Generate: FanOut parallel slide creation + sequential for dependent slides
/// ④ Review & Export: Quality gate loop → merge → PPTX export → notify
pub fn slide_creator() -> Workflow {
    Workflow::new(
        "slide_creator",
        "AI Slide Creator — Nghiên cứu → Lập kế hoạch → Tạo slide song song → Xuất PPTX",
    )
    .with_tags(vec!["slides", "presentation", "pptx", "research", "parallel"])
    .with_timeout(1800) // 30 min max for full pipeline
    // Stage 1: Research — thu thập dữ liệu
    .add_step(
        WorkflowStep::new("research", "researcher", StepType::Sequential)
            .with_prompt(
                "Bạn là chuyên gia nghiên cứu. Nhiệm vụ: nghiên cứu chủ đề sau để chuẩn bị tạo bài thuyết trình.\n\n\
                Chủ đề: {{input}}\n\n\
                Hãy:\n\
                1. Thu thập dữ liệu, số liệu, xu hướng quan trọng\n\
                2. Xác định 5-8 điểm chính cần trình bày\n\
                3. Tìm ví dụ, case study minh hoạ\n\
                4. Ghi lại nguồn tham khảo\n\n\
                Trả về kết quả nghiên cứu chi tiết, có cấu trúc rõ ràng."
            )
            .with_timeout(600)
            .with_retries(1),
    )
    // Stage 2: Plan — lập kế hoạch slide
    .add_step(
        WorkflowStep::new("plan", "planner", StepType::Sequential)
            .with_prompt(
                "Bạn là chuyên gia thiết kế bài thuyết trình. Dựa trên nghiên cứu sau, hãy lập kế hoạch chi tiết cho bài slide.\n\n\
                Nghiên cứu:\n{{input}}\n\n\
                Hãy tạo outline gồm:\n\
                1. Tiêu đề bài thuyết trình\n\
                2. Danh sách slides (10-20 slides), mỗi slide gồm:\n\
                   - Số thứ tự\n\
                   - Tiêu đề slide\n\
                   - Nội dung chính (3-5 bullet points)\n\
                   - Gợi ý hình ảnh/biểu đồ\n\
                   - Loại: cover, content, data, chart, quote, closing\n\
                3. Phân loại slides nào có thể tạo song song (độc lập) vs tuần tự (phụ thuộc)\n\n\
                Format kết quả rõ ràng để AI khác có thể tạo từng slide."
            )
            .with_timeout(300),
    )
    // Stage 3a: Generate slides — parallel FanOut
    .add_step(
        WorkflowStep::new("gen-intro", "slide-designer", StepType::Sequential)
            .with_prompt(
                "Tạo nội dung chi tiết cho SLIDE MỞ ĐẦU (Cover + Giới thiệu) dựa trên plan sau.\n\
                Viết nội dung đầy đủ, chuyên nghiệp cho mỗi slide.\n\n{{input}}"
            )
            .with_timeout(300),
    )
    .add_step(
        WorkflowStep::new("gen-body-a", "slide-designer", StepType::Sequential)
            .with_prompt(
                "Tạo nội dung chi tiết cho NHÓM SLIDE PHẦN 1 (slides 3-6 trong plan) dựa trên plan.\n\
                Viết nội dung đầy đủ, chuyên nghiệp, có số liệu cụ thể.\n\n{{input}}"
            )
            .with_timeout(300),
    )
    .add_step(
        WorkflowStep::new("gen-body-b", "slide-designer", StepType::Sequential)
            .with_prompt(
                "Tạo nội dung chi tiết cho NHÓM SLIDE PHẦN 2 (slides 7-10 trong plan) dựa trên plan.\n\
                Viết nội dung đầy đủ, chuyên nghiệp, có ví dụ minh hoạ.\n\n{{input}}"
            )
            .with_timeout(300),
    )
    .add_step(
        WorkflowStep::new("gen-closing", "slide-designer", StepType::Sequential)
            .with_prompt(
                "Tạo nội dung chi tiết cho SLIDE KẾT LUẬN (Tổng kết + CTA + Q&A) dựa trên plan.\n\
                Viết kết luận mạnh mẽ, có call-to-action rõ ràng.\n\n{{input}}"
            )
            .with_timeout(300),
    )
    // Stage 3b: FanOut — chạy song song 4 nhóm slide
    .add_step(WorkflowStep::new(
        "parallel-gen",
        "orchestrator",
        StepType::FanOut {
            parallel_steps: vec![
                "gen-intro".into(),
                "gen-body-a".into(),
                "gen-body-b".into(),
                "gen-closing".into(),
            ],
        },
    ))
    // Stage 3c: Collect — gom kết quả
    .add_step(WorkflowStep::new(
        "assemble",
        "orchestrator",
        StepType::Collect {
            strategy: CollectStrategy::Merge,
            evaluator: None,
        },
    ))
    // Stage 4a: Quality review loop
    .add_step(WorkflowStep::new(
        "quality-check",
        "quality-reviewer",
        StepType::Loop {
            body_step: "assemble".into(),
            config: LoopConfig::new(
                2, // max 2 revision rounds
                Condition::new("quality", "contains", "APPROVED"),
            ),
        },
    ))
    // Stage 4b: Export — format final output
    .add_step(
        WorkflowStep::new(
            "export",
            "formatter",
            StepType::Transform {
                template: "## 📊 Bài Thuyết Trình Hoàn Chỉnh\n\n\
                    Đã tạo xong! Dưới đây là nội dung đầy đủ của tất cả slides:\n\n\
                    {{input}}\n\n\
                    ---\n\
                    ✅ Workflow: Research → Plan → Generate (Parallel) → Review → Export\n\
                    📁 Sẵn sàng xuất ra PPTX bằng Document Generator skill."
                    .to_string(),
            },
        ),
    )
}

// ═══════════════════════════════════════════════════════════════
// CEO Workflows — Dành cho giám đốc doanh nghiệp SME
// ═══════════════════════════════════════════════════════════════

/// Meeting Recap: Audio/Text → Transcript → Summary → Action Items → Assign Tasks → Notify.
///
/// Input: Meeting notes hoặc transcript (copy/paste hoặc từ audio transcription).
/// Output: Biên bản họp + danh sách task + gửi thông báo đa kênh.
pub fn meeting_recap() -> Workflow {
    Workflow::new(
        "meeting_recap",
        "Meeting Recap — Biên bản họp → Tóm tắt → Tạo task → Gửi thông báo",
    )
    .with_tags(vec!["meeting", "recap", "tasks", "ceo", "management"])
    .with_timeout(900) // 15 min max
    // Step 1: Phân tích & tóm tắt nội dung họp
    .add_step(
        WorkflowStep::new("summarize", "meeting-analyst", StepType::Sequential)
            .with_prompt(
                "Bạn là thư ký hội đồng chuyên nghiệp. Phân tích nội dung cuộc họp sau và tạo biên bản:\n\n\
                Nội dung họp:\n{{input}}\n\n\
                Hãy tạo biên bản gồm:\n\
                1. **Thông tin cuộc họp**: Chủ đề, thời gian, thành phần tham dự (nếu có)\n\
                2. **Tóm tắt nội dung chính** (3-5 điểm)\n\
                3. **Các quyết định đã đưa ra** (liệt kê rõ ràng)\n\
                4. **Vấn đề chưa giải quyết** (nếu có)\n\
                5. **Số liệu/KPI được đề cập** (nếu có)"
            )
            .with_timeout(300)
            .with_retries(1),
    )
    // Step 2: Trích xuất action items & tasks
    .add_step(
        WorkflowStep::new("extract-tasks", "task-manager", StepType::Sequential)
            .with_prompt(
                "Từ biên bản họp sau, hãy trích xuất TẤT CẢ action items thành task list cụ thể:\n\n\
                {{input}}\n\n\
                Format mỗi task:\n\
                - **Task**: [Mô tả công việc]\n\
                - **Người phụ trách**: [Tên/Phòng ban]\n\
                - **Deadline**: [Ngày cụ thể hoặc khoảng thời gian]\n\
                - **Độ ưu tiên**: 🔴 Cao / 🟡 Trung bình / 🟢 Thấp\n\
                - **KPI đo lường**: [Tiêu chí hoàn thành]\n\n\
                Sắp xếp theo độ ưu tiên từ cao → thấp."
            )
            .with_timeout(300),
    )
    // Step 3: Tạo bản tóm tắt gửi team (ngắn gọn, dễ đọc)
    .add_step(
        WorkflowStep::new("team-summary", "communicator", StepType::Sequential)
            .with_prompt(
                "Viết bản tóm tắt cuộc họp ngắn gọn để gửi cho team qua Zalo/Telegram/Email.\n\n\
                Biên bản + Tasks:\n{{input}}\n\n\
                Format gửi team (ngắn gọn, thân thiện):\n\
                📋 **Kết quả họp [Chủ đề]**\n\
                ⏰ [Thời gian]\n\n\
                🎯 **Quyết định chính:**\n\
                • [bullet 1]\n\
                • [bullet 2]\n\n\
                📌 **Task phân công:**\n\
                • @[Tên] → [Task] — Deadline: [ngày]\n\
                • @[Tên] → [Task] — Deadline: [ngày]\n\n\
                ⚠️ **Lưu ý:** [nếu có]\n\n\
                Giữ đúng format, ngắn gọn, dễ đọc trên mobile."
            )
            .with_timeout(300),
    )
    // Step 4: Export format
    .add_step(
        WorkflowStep::new(
            "export",
            "formatter",
            StepType::Transform {
                template: "## 📋 Meeting Recap\n\n{{input}}\n\n---\n\
                    ✅ Auto-generated by BizClaw Meeting Recap Workflow\n\
                    📤 Sẵn sàng gửi qua Zalo / Telegram / Email"
                    .to_string(),
            },
        ),
    )
}

/// Daily CEO Briefing: Tổng hợp tin tức + KPIs + priorities mỗi sáng.
///
/// Chạy tự động lúc 7:00 sáng qua Scheduler.
/// Output: Bản briefing ngắn gọn gửi qua Zalo/Telegram.
pub fn ceo_daily_briefing() -> Workflow {
    Workflow::new(
        "ceo_daily_briefing",
        "CEO Daily Briefing — Tin tức thị trường + KPIs + Ưu tiên hôm nay",
    )
    .with_tags(vec!["ceo", "briefing", "daily", "kpi", "news"])
    .with_timeout(600)
    // Parallel: thu thập 3 nguồn thông tin cùng lúc
    .add_step(
        WorkflowStep::new("market-news", "market-analyst", StepType::Sequential)
            .with_prompt(
                "Thu thập 5 tin tức kinh doanh/thị trường quan trọng nhất hôm nay liên quan đến:\n\
                {{input}}\n\n\
                Mỗi tin gồm: Tiêu đề | Tóm tắt 1 dòng | Tác động đến doanh nghiệp (Tích cực/Tiêu cực/Trung lập)"
            )
            .with_timeout(300),
    )
    .add_step(
        WorkflowStep::new("kpi-check", "data-analyst", StepType::Sequential)
            .with_prompt(
                "Dựa trên ngành nghề sau, liệt kê các KPI quan trọng mà CEO cần theo dõi hàng ngày:\n\
                {{input}}\n\n\
                Format: KPI | Chỉ số mẫu | Xu hướng (↑↓→) | Hành động cần thiết\n\
                Gồm: Doanh thu, Chi phí, Leads, Conversion, Customer complaints, Cash flow"
            )
            .with_timeout(200),
    )
    .add_step(
        WorkflowStep::new("priorities", "strategy-advisor", StepType::Sequential)
            .with_prompt(
                "Dựa trên tin tức và KPIs, đề xuất 3 ưu tiên hàng đầu cho CEO hôm nay:\n\n\
                {{input}}\n\n\
                Format mỗi ưu tiên:\n\
                🎯 **Ưu tiên [N]**: [Tiêu đề]\n\
                - Lý do: [Tại sao quan trọng]\n\
                - Hành động: [Cụ thể cần làm gì]\n\
                - Thời gian: [Bao lâu]"
            )
            .with_timeout(200),
    )
    // FanOut: chạy song song 3 nguồn
    .add_step(WorkflowStep::new(
        "parallel-gather",
        "orchestrator",
        StepType::FanOut {
            parallel_steps: vec![
                "market-news".into(),
                "kpi-check".into(),
                "priorities".into(),
            ],
        },
    ))
    // Collect & merge
    .add_step(WorkflowStep::new(
        "merge",
        "orchestrator",
        StepType::Collect {
            strategy: CollectStrategy::Merge,
            evaluator: None,
        },
    ))
    // Format final briefing
    .add_step(
        WorkflowStep::new(
            "format",
            "formatter",
            StepType::Transform {
                template: "☀️ **CEO DAILY BRIEFING**\n\n{{input}}\n\n---\n\
                    🤖 Auto-generated by BizClaw | Chúc sếp một ngày hiệu quả!"
                    .to_string(),
            },
        ),
    )
}

/// Competitor Analysis: Research → Compare → SWOT → Strategy.
///
/// Input: Tên đối thủ hoặc ngành cần phân tích.
/// Output: Báo cáo phân tích đối thủ chi tiết.
pub fn competitor_analysis() -> Workflow {
    Workflow::new(
        "competitor_analysis",
        "Phân tích đối thủ — Nghiên cứu → So sánh → SWOT → Chiến lược",
    )
    .with_tags(vec!["competitor", "analysis", "strategy", "ceo", "market"])
    .with_timeout(1200)
    // Step 1: Thu thập thông tin đối thủ
    .add_step(
        WorkflowStep::new("research", "market-researcher", StepType::Sequential)
            .with_prompt(
                "Nghiên cứu chi tiết về đối thủ cạnh tranh:\n\n\
                {{input}}\n\n\
                Thu thập:\n\
                1. Thông tin công ty (năm thành lập, quy mô, doanh thu ước tính)\n\
                2. Sản phẩm/dịch vụ chính + Pricing\n\
                3. Điểm mạnh nổi bật\n\
                4. Điểm yếu / Complaints từ khách hàng\n\
                5. Chiến lược marketing (kênh, tần suất, tone)\n\
                6. Công nghệ / Nền tảng sử dụng",
            )
            .with_timeout(600)
            .with_retries(1),
    )
    // Step 2: So sánh & SWOT
    .add_step(
        WorkflowStep::new("compare-swot", "strategy-analyst", StepType::Sequential)
            .with_prompt(
                "Dựa trên nghiên cứu, tạo bảng so sánh và phân tích SWOT:\n\n\
                {{input}}\n\n\
                1. **Bảng so sánh**: [Tiêu chí | Chúng ta | Đối thủ | Ai thắng?]\n\
                   Gồm: Giá, Chất lượng, UX, Marketing, Support, Tech\n\
                2. **SWOT của đối thủ**:\n\
                   - Strengths (Điểm mạnh)\n\
                   - Weaknesses (Điểm yếu)\n\
                   - Opportunities (Cơ hội cho ta)\n\
                   - Threats (Mối đe doạ)",
            )
            .with_timeout(300),
    )
    // Step 3: Chiến lược đề xuất
    .add_step(
        WorkflowStep::new("strategy", "strategy-advisor", StepType::Sequential)
            .with_prompt(
                "Dựa trên phân tích đối thủ, đề xuất chiến lược cạnh tranh:\n\n\
                {{input}}\n\n\
                Đề xuất:\n\
                1. **Quick Wins** (thực hiện ngay, 1-2 tuần):\n\
                2. **Short-term** (1-3 tháng):\n\
                3. **Long-term** (6-12 tháng):\n\
                4. **Differentiation**: Điểm khác biệt nên tập trung\n\
                5. **Pricing strategy**: Nên cạnh tranh giá hay chất lượng?\n\
                6. **Marketing counter**: Cách đáp trả marketing của đối thủ",
            )
            .with_timeout(300),
    )
    // Export
    .add_step(WorkflowStep::new(
        "report",
        "formatter",
        StepType::Transform {
            template: "## 🏢 Báo Cáo Phân Tích Đối Thủ\n\n{{input}}\n\n---\n\
                    📊 Auto-generated by BizClaw Competitor Analysis"
                .to_string(),
        },
    ))
}

/// Proposal Generator: Client brief → Research → Draft → Review → Send.
///
/// Input: Yêu cầu/brief từ khách hàng.
/// Output: Proposal/báo giá chuyên nghiệp.
pub fn proposal_generator() -> Workflow {
    Workflow::new(
        "proposal_generator",
        "Tạo Proposal — Brief KH → Nghiên cứu → Soạn → Duyệt → Gửi",
    )
    .with_tags(vec!["proposal", "sales", "quote", "ceo", "client"])
    .with_timeout(1200)
    // Step 1: Phân tích brief khách hàng
    .add_step(
        WorkflowStep::new("analyze-brief", "sales-analyst", StepType::Sequential)
            .with_prompt(
                "Phân tích yêu cầu/brief từ khách hàng:\n\n\
                {{input}}\n\n\
                Xác định:\n\
                1. Nhu cầu chính của KH\n\
                2. Budget ước tính (nếu đề cập)\n\
                3. Timeline mong muốn\n\
                4. Tiêu chí lựa chọn nhà cung cấp\n\
                5. Pain points / vấn đề đang gặp\n\
                6. Đề xuất giải pháp phù hợp",
            )
            .with_timeout(300),
    )
    // Step 2: Soạn proposal
    .add_step(
        WorkflowStep::new("draft-proposal", "proposal-writer", StepType::Sequential)
            .with_prompt(
                "Soạn proposal chuyên nghiệp dựa trên phân tích sau:\n\n\
                {{input}}\n\n\
                Cấu trúc proposal:\n\
                1. **Executive Summary** (tóm tắt cho CEO đọc nhanh)\n\
                2. **Hiểu biết về nhu cầu** (cho KH thấy ta hiểu họ)\n\
                3. **Giải pháp đề xuất** (chi tiết, rõ ràng)\n\
                4. **Bảng giá** (packages nếu có, so sánh options)\n\
                5. **Timeline triển khai** (milestones)\n\
                6. **Cam kết & SLA**\n\
                7. **Về chúng tôi** (credentials, case studies)\n\
                8. **Bước tiếp theo** (CTA rõ ràng)\n\n\
                Tone: Chuyên nghiệp, tự tin, hướng đến giải quyết vấn đề.",
            )
            .with_timeout(600)
            .with_retries(1),
    )
    // Step 3: Review quality
    .add_step(WorkflowStep::new(
        "quality-gate",
        "reviewer",
        StepType::Loop {
            body_step: "draft-proposal".into(),
            config: LoopConfig::new(2, Condition::new("quality", "contains", "APPROVED")),
        },
    ))
    // Step 4: Format final
    .add_step(WorkflowStep::new(
        "finalize",
        "formatter",
        StepType::Transform {
            template: "## 📄 Proposal\n\n{{input}}\n\n---\n\
                    ✅ Ready to send | Auto-generated by BizClaw"
                .to_string(),
        },
    ))
}

/// Weekly Report: Collect → Synthesize → Format → Distribute.
///
/// Input: Tên công ty/phòng ban cần tổng hợp.
/// Output: Báo cáo tuần gửi qua đa kênh.
pub fn weekly_report() -> Workflow {
    Workflow::new(
        "weekly_report",
        "Báo Cáo Tuần — Thu thập → Tổng hợp → Format → Phân phối",
    )
    .with_tags(vec!["report", "weekly", "kpi", "ceo", "management"])
    .with_timeout(900)
    // Parallel: thu thập báo cáo từ nhiều góc
    .add_step(
        WorkflowStep::new("sales-report", "sales-analyst", StepType::Sequential)
            .with_prompt(
                "Tạo phần BÁO CÁO KINH DOANH trong tuần cho:\n\n\
                {{input}}\n\n\
                Gồm: Doanh thu tuần, So sánh tuần trước, Top deals, Pipeline, Dự báo tháng",
            )
            .with_timeout(200),
    )
    .add_step(
        WorkflowStep::new("ops-report", "ops-analyst", StepType::Sequential)
            .with_prompt(
                "Tạo phần BÁO CÁO VẬN HÀNH trong tuần cho:\n\n\
                {{input}}\n\n\
                Gồm: Tiến độ dự án, Issues phát sinh, Resource utilization, Customer incidents",
            )
            .with_timeout(200),
    )
    .add_step(
        WorkflowStep::new("finance-report", "finance-analyst", StepType::Sequential)
            .with_prompt(
                "Tạo phần BÁO CÁO TÀI CHÍNH trong tuần cho:\n\n\
                {{input}}\n\n\
                Gồm: Cash flow, Chi phí, Accounts receivable, Budget vs Actual, Highlights",
            )
            .with_timeout(200),
    )
    // FanOut parallel
    .add_step(WorkflowStep::new(
        "parallel-collect",
        "orchestrator",
        StepType::FanOut {
            parallel_steps: vec![
                "sales-report".into(),
                "ops-report".into(),
                "finance-report".into(),
            ],
        },
    ))
    // Collect & merge
    .add_step(WorkflowStep::new(
        "merge",
        "orchestrator",
        StepType::Collect {
            strategy: CollectStrategy::Merge,
            evaluator: None,
        },
    ))
    // Synthesize: CEO executive summary
    .add_step(
        WorkflowStep::new("executive-summary", "ceo-advisor", StepType::Sequential)
            .with_prompt(
                "Từ báo cáo các phòng ban, viết EXECUTIVE SUMMARY cho CEO:\n\n\
                {{input}}\n\n\
                Format:\n\
                📊 **WEEKLY EXECUTIVE SUMMARY**\n\n\
                🟢 **Highlights** (3 điểm tốt nhất tuần)\n\
                🔴 **Concerns** (2-3 vấn đề cần chú ý)\n\
                🎯 **Focus tuần tới** (3 ưu tiên)\n\
                📈 **KPIs**: [bảng KPI ngắn gọn]\n\n\
                Ngắn gọn, đi thẳng vào vấn đề, CEO đọc trong 2 phút.",
            )
            .with_timeout(300),
    )
    // Export
    .add_step(WorkflowStep::new(
        "export",
        "formatter",
        StepType::Transform {
            template: "## 📊 Báo Cáo Tuần\n\n{{input}}\n\n---\n\
                    🤖 Auto-generated by BizClaw Weekly Report Workflow"
                .to_string(),
        },
    ))
}

// ═══════════════════════════════════════════════════════════════
// Business Operations Workflows
// ═══════════════════════════════════════════════════════════════

/// Email Drip Campaign: Soạn chuỗi email nurturing tự động.
///
/// Input: Mô tả sản phẩm/dịch vụ + đối tượng khách hàng.
/// Output: Chuỗi 5 email chuyên nghiệp.
pub fn email_drip_campaign() -> Workflow {
    Workflow::new(
        "email_drip_campaign",
        "Email Drip Campaign — Soạn chuỗi 5 email nurturing tự động",
    )
    .with_tags(vec!["email", "marketing", "drip", "nurturing", "sales"])
    .with_timeout(1200)
    .add_step(
        WorkflowStep::new(
            "audience-research",
            "marketing-analyst",
            StepType::Sequential,
        )
        .with_prompt(
            "Phân tích đối tượng khách hàng và hành trình mua hàng cho:\n\n\
                {{input}}\n\n\
                Xác định:\n\
                1. Persona chính (demographics, pain points, goals)\n\
                2. Hành trình mua hàng (Awareness → Interest → Decision → Action)\n\
                3. Objections thường gặp\n\
                4. Trigger events (khi nào KH cần sản phẩm nhất)\n\
                5. Tone & style phù hợp",
        )
        .with_timeout(300),
    )
    .add_step(
        WorkflowStep::new("email-1", "copywriter", StepType::Sequential)
            .with_prompt(
                "Viết Email #1 — WELCOME/AWARENESS (ngày 0):\n\n\
                {{input}}\n\n\
                - Subject line hấp dẫn (A/B: 2 variants)\n\
                - Giới thiệu vấn đề KH đang gặp\n\
                - Hint về giải pháp (nhưng chưa bán)\n\
                - CTA: Đọc thêm / Xem video\n\
                Tone: Thân thiện, đồng cảm, không bán hàng.",
            )
            .with_timeout(200),
    )
    .add_step(
        WorkflowStep::new("email-2", "copywriter", StepType::Sequential)
            .with_prompt(
                "Viết Email #2 — VALUE/EDUCATION (ngày 3):\n\n\
                {{input}}\n\n\
                - Chia sẻ kiến thức hữu ích (tips, case study)\n\
                - Chứng minh expertise\n\
                - Social proof nhẹ nhàng\n\
                - CTA: Download guide / Tham gia webinar",
            )
            .with_timeout(200),
    )
    .add_step(
        WorkflowStep::new("email-3", "copywriter", StepType::Sequential)
            .with_prompt(
                "Viết Email #3 — SOCIAL PROOF (ngày 7):\n\n\
                {{input}}\n\n\
                - Case study thực tế (before/after)\n\
                - Testimonials\n\
                - Số liệu kết quả cụ thể\n\
                - CTA: Xem demo / Dùng thử",
            )
            .with_timeout(200),
    )
    .add_step(
        WorkflowStep::new("email-4", "copywriter", StepType::Sequential)
            .with_prompt(
                "Viết Email #4 — OFFER (ngày 10):\n\n\
                {{input}}\n\n\
                - Giới thiệu sản phẩm/dịch vụ chính thức\n\
                - Bảng giá rõ ràng + packages\n\
                - Ưu đãi giới hạn thời gian\n\
                - FAQ ngắn (xử lý objections)\n\
                - CTA: Đăng ký / Mua ngay",
            )
            .with_timeout(200),
    )
    .add_step(
        WorkflowStep::new("email-5", "copywriter", StepType::Sequential)
            .with_prompt(
                "Viết Email #5 — URGENCY/LAST CHANCE (ngày 14):\n\n\
                {{input}}\n\n\
                - Nhắc ưu đãi sắp hết\n\
                - Tóm tắt lý do nên hành động\n\
                - Bonus cho người đăng ký sớm\n\
                - CTA mạnh: Hành động ngay\n\
                Tone: Urgency nhưng không spam.",
            )
            .with_timeout(200),
    )
    // FanOut: tạo email 2-5 song song (email 1 đã có)
    .add_step(WorkflowStep::new(
        "parallel-emails",
        "orchestrator",
        StepType::FanOut {
            parallel_steps: vec![
                "email-2".into(),
                "email-3".into(),
                "email-4".into(),
                "email-5".into(),
            ],
        },
    ))
    .add_step(WorkflowStep::new(
        "assemble",
        "orchestrator",
        StepType::Collect {
            strategy: CollectStrategy::Merge,
            evaluator: None,
        },
    ))
    .add_step(WorkflowStep::new(
        "export",
        "formatter",
        StepType::Transform {
            template: "## 📧 Email Drip Campaign (5 emails)\n\n{{input}}\n\n---\n\
                    ✅ Ready to schedule | Auto-generated by BizClaw"
                .to_string(),
        },
    ))
}

/// Hiring Pipeline: JD → Screen → Interview Questions → Evaluation.
///
/// Input: Vị trí cần tuyển + yêu cầu.
/// Output: JD + câu hỏi phỏng vấn + bảng đánh giá.
pub fn hiring_pipeline() -> Workflow {
    Workflow::new(
        "hiring_pipeline",
        "Tuyển dụng — JD → Câu hỏi PV → Ma trận đánh giá → Onboarding checklist",
    )
    .with_tags(vec!["hiring", "hr", "recruitment", "interview", "ceo"])
    .with_timeout(900)
    .add_step(
        WorkflowStep::new("job-description", "hr-specialist", StepType::Sequential)
            .with_prompt(
                "Soạn JD (Job Description) chuyên nghiệp cho vị trí:\n\n\
                {{input}}\n\n\
                Gồm:\n\
                1. Giới thiệu công ty (2-3 dòng hấp dẫn)\n\
                2. Mô tả công việc (5-7 responsibilities)\n\
                3. Yêu cầu bắt buộc (must-have skills)\n\
                4. Yêu cầu ưu tiên (nice-to-have)\n\
                5. Quyền lợi (salary range, benefits, culture)\n\
                6. Quy trình ứng tuyển\n\n\
                Tone: Chuyên nghiệp, hấp dẫn, thể hiện văn hoá công ty.",
            )
            .with_timeout(300),
    )
    .add_step(
        WorkflowStep::new("interview-questions", "hr-specialist", StepType::Sequential)
            .with_prompt(
                "Tạo bộ câu hỏi phỏng vấn cho vị trí trong JD sau:\n\n\
                {{input}}\n\n\
                Gồm 3 vòng:\n\
                **Vòng 1 — Culture Fit (HR, 30 phút):**\n\
                - 5 câu hỏi soft skills + culture\n\n\
                **Vòng 2 — Technical (Hiring Manager, 45 phút):**\n\
                - 5 câu hỏi chuyên môn + case study\n\
                - 1 bài test thực tế (nếu applicable)\n\n\
                **Vòng 3 — Final (CEO/Director, 20 phút):**\n\
                - 3 câu hỏi vision, growth, salary expectation\n\n\
                Mỗi câu gồm: Câu hỏi | Tiêu chí đánh giá | Red flags",
            )
            .with_timeout(300),
    )
    .add_step(
        WorkflowStep::new("eval-matrix", "hr-specialist", StepType::Sequential)
            .with_prompt(
                "Tạo ma trận đánh giá ứng viên dựa trên JD + câu hỏi:\n\n\
                {{input}}\n\n\
                Format bảng:\n\
                | Tiêu chí | Trọng số | Đánh giá (1-5) | Ghi chú |\n\
                |----------|----------|----------------|---------|\n\
                Gồm: Technical skills, Soft skills, Culture fit, Experience, Growth potential\n\n\
                + Thang điểm: <15 = Reject, 15-20 = Maybe, >20 = Hire\n\
                + Onboarding checklist 30 ngày đầu tiên",
            )
            .with_timeout(300),
    )
    .add_step(WorkflowStep::new(
        "export",
        "formatter",
        StepType::Transform {
            template: "## 👥 Hiring Pipeline\n\n{{input}}\n\n---\n\
                    ✅ JD + Interview Questions + Evaluation Matrix\n\
                    🤖 Auto-generated by BizClaw"
                .to_string(),
        },
    ))
}

/// Customer Feedback Analysis: Collect → Categorize → Insights → Action Plan.
///
/// Input: Danh sách feedback/reviews từ khách hàng.
/// Output: Phân tích + insights + action plan.
pub fn customer_feedback_analysis() -> Workflow {
    Workflow::new(
        "customer_feedback",
        "Phân tích Feedback KH — Thu thập → Phân loại → Insights → Hành động",
    )
    .with_tags(vec![
        "feedback",
        "customer",
        "analysis",
        "csat",
        "improvement",
    ])
    .with_timeout(900)
    .add_step(
        WorkflowStep::new("categorize", "analyst", StepType::Sequential)
            .with_prompt(
                "Phân tích và phân loại feedback khách hàng sau:\n\n\
                {{input}}\n\n\
                Phân loại theo:\n\
                1. **Sentiment**: 😊 Tích cực / 😐 Trung lập / 😠 Tiêu cực (% mỗi loại)\n\
                2. **Chủ đề**: Sản phẩm, Dịch vụ, Giá, UI/UX, Support, Delivery\n\
                3. **Mức độ nghiêm trọng**: 🔴 Critical / 🟡 Medium / 🟢 Low\n\
                4. **Tần suất**: Vấn đề nào được nhắc nhiều nhất?\n\
                5. **NPS ước tính** (nếu có đủ data)",
            )
            .with_timeout(300),
    )
    .add_step(
        WorkflowStep::new("insights", "strategy-analyst", StepType::Sequential)
            .with_prompt(
                "Từ phân tích feedback, rút ra insights kinh doanh:\n\n\
                {{input}}\n\n\
                1. **Top 3 điểm mạnh** (giữ vững & phát huy)\n\
                2. **Top 3 điểm yếu** (cần cải thiện ngay)\n\
                3. **Cơ hội** (từ feedback tích cực → upsell/cross-sell)\n\
                4. **Rủi ro churn** (KH nào có nguy cơ rời bỏ?)\n\
                5. **So sánh vs kỳ trước** (tốt hơn hay xấu đi?)",
            )
            .with_timeout(300),
    )
    .add_step(
        WorkflowStep::new("action-plan", "operations-manager", StepType::Sequential)
            .with_prompt(
                "Tạo action plan từ insights feedback KH:\n\n\
                {{input}}\n\n\
                Format:\n\
                🔴 **Urgent (tuần này)**:\n\
                - [Action] → [Phòng ban] → [KPI đo lường]\n\n\
                🟡 **Short-term (tháng này)**:\n\
                - [Action] → [Phòng ban] → [KPI]\n\n\
                🟢 **Long-term (quý này)**:\n\
                - [Action] → [Phòng ban] → [KPI]\n\n\
                📊 **KPI theo dõi**: CSAT target, Response time, Resolution rate",
            )
            .with_timeout(300),
    )
    .add_step(WorkflowStep::new(
        "export",
        "formatter",
        StepType::Transform {
            template: "## 📊 Customer Feedback Report\n\n{{input}}\n\n---\n\
                    🤖 Auto-generated by BizClaw Feedback Analysis"
                .to_string(),
        },
    ))
}

/// Contract Review: Đọc HĐ → Rủi ro → Đề xuất sửa → Tóm tắt.
///
/// Input: Nội dung hợp đồng (copy/paste hoặc tóm tắt điều khoản).
/// Output: Phân tích rủi ro + đề xuất sửa + tóm tắt.
pub fn contract_review() -> Workflow {
    Workflow::new(
        "contract_review",
        "Review Hợp đồng — Đọc → Rủi ro pháp lý → Đề xuất sửa → Tóm tắt",
    )
    .with_tags(vec!["contract", "legal", "review", "risk", "ceo"])
    .with_timeout(900)
    .add_step(
        WorkflowStep::new("analyze", "legal-analyst", StepType::Sequential)
            .with_prompt(
                "Phân tích hợp đồng sau từ góc nhìn pháp lý:\n\n\
                {{input}}\n\n\
                Kiểm tra:\n\
                1. **Các bên**: Thông tin đầy đủ?\n\
                2. **Phạm vi công việc/dịch vụ**: Rõ ràng không?\n\
                3. **Giá & thanh toán**: Điều khoản thanh toán, phạt trễ\n\
                4. **Thời hạn & gia hạn**: Tự động gia hạn?\n\
                5. **Bảo mật & NDA**: Có điều khoản?\n\
                6. **Bồi thường & trách nhiệm**: Giới hạn?\n\
                7. **Chấm dứt**: Điều kiện, thông báo trước\n\
                8. **Tranh chấp**: Phương thức giải quyết\n\
                9. **Điều khoản bất lợi**: Bẫy, lock-in, penalty",
            )
            .with_timeout(300),
    )
    .add_step(
        WorkflowStep::new("risk-assess", "risk-analyst", StepType::Sequential)
            .with_prompt(
                "Đánh giá rủi ro từ phân tích hợp đồng:\n\n\
                {{input}}\n\n\
                Format:\n\
                🔴 **Rủi ro CAO** (cần sửa trước khi ký):\n\
                - [Điều khoản] → [Rủi ro] → [Đề xuất sửa cụ thể]\n\n\
                🟡 **Rủi ro TRUNG BÌNH** (nên negotiate):\n\
                - [Điều khoản] → [Rủi ro] → [Đề xuất]\n\n\
                🟢 **OK** (chấp nhận được):\n\
                - [Liệt kê]\n\n\
                📋 **Kết luận**: Nên ký / Cần sửa / Không nên ký",
            )
            .with_timeout(300),
    )
    .add_step(
        WorkflowStep::new("summary", "legal-writer", StepType::Sequential)
            .with_prompt(
                "Viết tóm tắt hợp đồng ngắn gọn cho CEO (đọc trong 2 phút):\n\n\
                {{input}}\n\n\
                Format:\n\
                📝 **Tóm tắt HĐ**: [Tên HĐ]\n\
                👥 Đối tác: [Tên]\n\
                💰 Giá trị: [Số tiền]\n\
                ⏰ Thời hạn: [Ngày]\n\
                ⚠️ Rủi ro: [Tóm tắt 1 dòng]\n\
                ✅ Đề xuất: [Ký / Sửa / Không ký]\n\
                📌 Cần sửa: [2-3 điểm quan trọng nhất]",
            )
            .with_timeout(200),
    )
    .add_step(WorkflowStep::new(
        "export",
        "formatter",
        StepType::Transform {
            template: "## ⚖️ Contract Review Report\n\n{{input}}\n\n---\n\
                    ⚠️ Đây là phân tích AI, không thay thế tư vấn pháp lý chuyên nghiệp.\n\
                    🤖 Auto-generated by BizClaw"
                .to_string(),
        },
    ))
}

/// Product Launch Checklist: Research → Plan → Content → PR → Track.
///
/// Input: Sản phẩm/tính năng sắp ra mắt.
/// Output: Checklist + content plan + PR draft.
pub fn product_launch_checklist() -> Workflow {
    Workflow::new(
        "product_launch",
        "Ra mắt sản phẩm — Research → Marketing Plan → Content → PR → Tracking",
    )
    .with_tags(vec!["launch", "product", "marketing", "pr", "ceo"])
    .with_timeout(1200)
    .add_step(
        WorkflowStep::new("market-research", "researcher", StepType::Sequential)
            .with_prompt(
                "Nghiên cứu thị trường cho sản phẩm sắp ra mắt:\n\n\
                {{input}}\n\n\
                Phân tích:\n\
                1. Thị trường mục tiêu (TAM, SAM, SOM)\n\
                2. Đối thủ có sản phẩm tương tự\n\
                3. Differentiation (ta khác gì?)\n\
                4. Pricing benchmark\n\
                5. Kênh phân phối phù hợp\n\
                6. Timing: Thời điểm tốt nhất ra mắt",
            )
            .with_timeout(300),
    )
    .add_step(
        WorkflowStep::new("launch-plan", "marketing-strategist", StepType::Sequential)
            .with_prompt(
                "Lập kế hoạch ra mắt chi tiết dựa trên research:\n\n\
                {{input}}\n\n\
                Kế hoạch gồm:\n\
                📅 **T-14 ngày**: Teaser campaign, landing page\n\
                📅 **T-7 ngày**: Early access, influencer outreach\n\
                📅 **D-Day**: Launch announcement multi-channel\n\
                📅 **T+7 ngày**: Follow-up, early feedback, PR\n\
                📅 **T+30 ngày**: Review, optimize, scale\n\n\
                Mỗi phase gồm: Tasks, Owner, KPI, Budget",
            )
            .with_timeout(300),
    )
    // Parallel: tạo content + PR cùng lúc
    .add_step(
        WorkflowStep::new("content-pack", "content-creator", StepType::Sequential)
            .with_prompt(
                "Tạo content package cho launch event:\n\n\
                {{input}}\n\n\
                Gồm:\n\
                1. **Headline** (5 variants cho A/B testing)\n\
                2. **Product description** (50, 150, 500 words)\n\
                3. **Social media posts** (Facebook, LinkedIn, Twitter)\n\
                4. **Email announcement**\n\
                5. **Landing page copy** (Hero, Features, CTA)",
            )
            .with_timeout(300),
    )
    .add_step(
        WorkflowStep::new("pr-draft", "pr-specialist", StepType::Sequential)
            .with_prompt(
                "Soạn PR materials cho ra mắt sản phẩm:\n\n\
                {{input}}\n\n\
                Gồm:\n\
                1. **Press Release** (format chuẩn báo chí)\n\
                2. **Media Kit** (key facts, quotes, images cần)\n\
                3. **Pitch email** cho nhà báo/blogger\n\
                4. **FAQ** (10 câu hỏi thường gặp)\n\
                5. **Talking points** cho CEO khi phỏng vấn",
            )
            .with_timeout(300),
    )
    .add_step(WorkflowStep::new(
        "parallel-create",
        "orchestrator",
        StepType::FanOut {
            parallel_steps: vec!["content-pack".into(), "pr-draft".into()],
        },
    ))
    .add_step(WorkflowStep::new(
        "merge",
        "orchestrator",
        StepType::Collect {
            strategy: CollectStrategy::Merge,
            evaluator: None,
        },
    ))
    .add_step(WorkflowStep::new(
        "export",
        "formatter",
        StepType::Transform {
            template: "## 🚀 Product Launch Kit\n\n{{input}}\n\n---\n\
                    ✅ Launch Plan + Content Pack + PR Materials\n\
                    🤖 Auto-generated by BizClaw"
                .to_string(),
        },
    ))
}

// ═══════════════════════════════════════════════════════════════
// Agent Team Workflows — Micro SaaS Operations
// ═══════════════════════════════════════════════════════════════

/// Vigor TrendScout — scan trends every 2h, analyze, report to Max.
///
/// Runs on cron `0 */2 * * *`. Agent: Vigor (Gemini Flash).
/// Output: Trend report with scores, sent to Max for synthesis.
pub fn vigor_trend_scout() -> Workflow {
    Workflow::new(
        "vigor_trend_scout",
        "TrendScout — Scan Product Hunt · HN · Reddit · Twitter → Score → Report",
    )
    .with_tags(vec!["growth", "trends", "vigor", "agent_team", "automated"])
    .with_timeout(600)
    // Parallel scan multiple platforms
    .add_step(
        WorkflowStep::new("scan-producthunt", "vigor", StepType::Sequential)
            .with_prompt(
                "Scan Product Hunt for the latest trending products in our SaaS category.\n\
                 Focus on: AI tools, automation, developer tools, business ops.\n\n\
                 For each relevant product, report:\n\
                 - Name & tagline\n\
                 - Upvote count\n\
                 - Relevance score (1-10) to our Micro SaaS\n\
                 - Key takeaway or competitive insight\n\n\
                 Context: {{input}}",
            )
            .with_timeout(180),
    )
    .add_step(
        WorkflowStep::new("scan-hackernews", "vigor", StepType::Sequential)
            .with_prompt(
                "Scan Hacker News front page and recent Show HN posts.\n\
                 Focus on: SaaS launches, AI agent discussions, developer tool trends.\n\n\
                 For each relevant thread, report:\n\
                 - Title & points\n\
                 - Key discussion themes\n\
                 - Relevance score (1-10)\n\
                 - Actionable insight for our product\n\n\
                 Context: {{input}}",
            )
            .with_timeout(180),
    )
    .add_step(
        WorkflowStep::new("scan-reddit", "vigor", StepType::Sequential)
            .with_prompt(
                "Scan Reddit communities: r/SaaS, r/Entrepreneur, r/startups, r/IndieHackers.\n\
                 Focus on: trending discussions, pain points, product recommendations.\n\n\
                 For each relevant post:\n\
                 - Subreddit, title, upvotes\n\
                 - Key pain point or opportunity\n\
                 - Relevance score (1-10)\n\
                 - Potential content/engagement opportunity\n\n\
                 Context: {{input}}",
            )
            .with_timeout(180),
    )
    // FanOut: scan all 3 platforms in parallel
    .add_step(WorkflowStep::new(
        "parallel-scan",
        "max",
        StepType::FanOut {
            parallel_steps: vec![
                "scan-producthunt".into(),
                "scan-hackernews".into(),
                "scan-reddit".into(),
            ],
        },
    ))
    // Collect results
    .add_step(WorkflowStep::new(
        "merge-trends",
        "max",
        StepType::Collect {
            strategy: CollectStrategy::Merge,
            evaluator: None,
        },
    ))
    // Synthesize into scored report
    .add_step(
        WorkflowStep::new("synthesize", "vigor", StepType::Sequential)
            .with_prompt(
                "Synthesize all trend data into a single TrendScout report.\n\n\
                 Raw data:\n{{input}}\n\n\
                 Create the report in this format:\n\
                 🔍 TREND SCOUT — [Current Time]\n\n\
                 🔥 HOT TRENDS (Score ≥ 7)\n\
                 1. [Trend] — Score: [X] — Source: [Platform]\n\
                    Action: [What we should do]\n\n\
                 📊 MONITORING (Score 4-6)\n\
                 • [Trend] — Score: [X] — [Brief note]\n\n\
                 💡 CONTENT OPPORTUNITIES\n\
                 • [Blog/social post idea based on trends]\n\n\
                 ⚡ COMPETITOR ALERTS\n\
                 • [Any competitor launches or major moves]",
            )
            .with_timeout(180),
    )
}

/// Vigor Blog Pipeline — research keyword → draft → SEO optimize → review.
///
/// Agent: Vigor (Gemini Flash). Human-triggered or scheduled.
/// Output: SEO-optimized blog post ready for publishing.
pub fn vigor_blog_pipeline() -> Workflow {
    Workflow::new(
        "vigor_blog_pipeline",
        "Blog Pipeline — Keyword Research → Draft → SEO Optimize → Quality Review",
    )
    .with_tags(vec![
        "growth",
        "blog",
        "seo",
        "vigor",
        "agent_team",
        "content",
    ])
    .with_timeout(1200)
    .add_step(
        WorkflowStep::new("keyword-research", "vigor", StepType::Sequential)
            .with_prompt(
                "Research SEO keywords for a blog post about:\n\n\
                 {{input}}\n\n\
                 Identify:\n\
                 1. Primary keyword (search volume 100-1000/mo, low-medium competition)\n\
                 2. Secondary keywords (3-5 related terms)\n\
                 3. Long-tail variations (5-10)\n\
                 4. Search intent analysis (informational/transactional/navigational)\n\
                 5. Competitor content analysis (top 3 ranking pages)\n\
                 6. Content gap opportunities\n\
                 7. Recommended word count and structure",
            )
            .with_timeout(300)
            .with_retries(1),
    )
    .add_step(
        WorkflowStep::new("draft", "vigor", StepType::Sequential)
            .with_prompt(
                "Write a comprehensive blog post based on this keyword research:\n\n\
                 {{input}}\n\n\
                 Requirements:\n\
                 - 1200-2000 words\n\
                 - Clear H1, H2, H3 hierarchy\n\
                 - Include primary keyword in H1, first paragraph, and 2-3 H2s\n\
                 - Natural keyword density (1-2%)\n\
                 - Actionable advice with specific examples\n\
                 - Include a FAQ section (3-5 questions) for featured snippets\n\
                 - End with a clear CTA related to our product\n\
                 - Tone: Professional, helpful, slightly casual (American English)\n\
                 - E-E-A-T compliant: show expertise and experience",
            )
            .with_timeout(600)
            .with_retries(1),
    )
    .add_step(
        WorkflowStep::new("seo-optimize", "vigor", StepType::Sequential)
            .with_prompt(
                "SEO-optimize this blog post draft:\n\n\
                 {{input}}\n\n\
                 Optimize:\n\
                 1. Meta title (50-60 chars, include primary keyword)\n\
                 2. Meta description (150-160 chars, compelling, include keyword)\n\
                 3. URL slug (short, keyword-rich)\n\
                 4. Internal link suggestions (2-3 relevant pages)\n\
                 5. Image alt text suggestions (3-5 images)\n\
                 6. Schema markup recommendation\n\
                 7. Readability score check (aim for Grade 8)\n\
                 8. Keyword placement audit\n\n\
                 Return the optimized post with all SEO metadata.",
            )
            .with_timeout(300),
    )
    // Quality review loop
    .add_step(WorkflowStep::new(
        "quality-gate",
        "max",
        StepType::Loop {
            body_step: "seo-optimize".into(),
            config: LoopConfig::new(2, Condition::new("quality", "contains", "APPROVED")),
        },
    ))
    .add_step(WorkflowStep::new(
        "publish-ready",
        "vigor",
        StepType::Transform {
            template: "## 📝 Blog Post Ready for Publishing\n\n\
                {{input}}\n\n\
                ---\n\
                ✅ SEO Optimized | Quality Reviewed\n\
                📊 Workflow: Keyword Research → Draft → SEO → Review\n\
                🤖 Generated by Vigor (Growth Agent)"
                .to_string(),
        },
    ))
}

/// Fidus Health Check — monitor instance, DB, disk, RAM, cache.
///
/// Runs every 5 minutes (Interval 300s). Agent: Fidus (DeepSeek V3).
/// Output: Health status report, alerts on critical issues.
pub fn fidus_health_check() -> Workflow {
    Workflow::new(
        "fidus_health_check",
        "Health Check — Instance · DB · Disk · RAM · Cache → Alert if Critical",
    )
    .with_tags(vec![
        "ops",
        "health",
        "monitoring",
        "fidus",
        "agent_team",
        "automated",
    ])
    .with_timeout(120)
    .add_step(
        WorkflowStep::new("check-infra", "fidus", StepType::Sequential)
            .with_prompt(
                "Perform a platform health check. Check the following:\n\n\
                 1. **Instance**: CPU usage, memory usage, process count, uptime\n\
                 2. **Database**: Connection pool status, query latency, table sizes\n\
                 3. **Disk**: Usage percentage, growth rate, estimated days to full\n\
                 4. **RAM**: Available vs used, swap usage, OOM risk\n\
                 5. **Cache**: Hit rate (ALERT if below 60%), eviction rate\n\
                 6. **Network**: Response times to key endpoints\n\n\
                 Report format:\n\
                 🔧 HEALTH CHECK — [Timestamp]\n\n\
                 Instance: [🟢/🟡/🔴] | CPU: [X]% | RAM: [X]/[X]GB\n\
                 Database: [🟢/🟡/🔴] | Conn: [X]/[X] | Latency: [X]ms\n\
                 Disk:     [🟢/🟡/🔴] | [X]% used | ~[X] days to full\n\
                 Cache:    [🟢/🟡/🔴] | Hit rate: [X]%\n\n\
                 ⚠️ ALERTS: [list any issues]\n\
                 ✅ ALL CLEAR [if none]\n\n\
                 Context: {{input}}",
            )
            .with_timeout(60),
    )
    .add_step(
        WorkflowStep::new("check-runaway", "fidus", StepType::Sequential)
            .with_prompt(
                "Check for runaway requests and anomalies:\n\n\
                 {{input}}\n\n\
                 1. Request count per endpoint in last 24h\n\
                 2. ALERT if any endpoint > 200 requests/day\n\
                 3. Identify source: bot traffic, retry storm, infinite loop, DDoS\n\
                 4. Recommendation (throttle, block, investigate)\n\n\
                 ⚠️ NEVER auto-block or restart without Max approval.\n\
                 Report anomalies only.",
            )
            .with_timeout(60),
    )
}

/// Fidus Cost Tracker — daily token cost report by model.
///
/// Runs daily at 23:00 (cron `0 23 * * *`). Agent: Fidus.
/// Output: Cost breakdown by model, comparison with yesterday, budget status.
pub fn fidus_cost_tracker() -> Workflow {
    Workflow::new(
        "fidus_cost_tracker",
        "Daily Cost Report — Token usage by model · Budget tracking · Anomaly detection",
    )
    .with_tags(vec![
        "ops",
        "cost",
        "budget",
        "fidus",
        "agent_team",
        "automated",
    ])
    .with_timeout(300)
    .add_step(
        WorkflowStep::new("collect-costs", "fidus", StepType::Sequential)
            .with_prompt(
                "Generate the daily token cost report.\n\n\
                 Context: {{input}}\n\n\
                 Report format:\n\
                 💰 DAILY COST REPORT — [Date]\n\n\
                 | Agent | Model | Tokens | Cost ($) |\n\
                 |-------|-------|--------|----------|\n\
                 | Max | Claude Sonnet 4 | [X]K | $[X] |\n\
                 | Vigor | Gemini Flash | [X]K | $[X] |\n\
                 | Fidus | DeepSeek V3 | [X]K | $[X] |\n\
                 | Optimo | Gemini Flash | [X]K | $[X] |\n\
                 | Mercury | GPT-4o Mini | [X]K | $[X] |\n\
                 | TOTAL | — | [X]M | $[X] |\n\n\
                 📊 vs Yesterday: [+/-X]% tokens, [+/-X]% cost\n\
                 📈 Month-to-date: $[X] / $[X] budget ([X]%)\n\n\
                 ⚠️ ANOMALIES: [cost spikes > 2x daily average]\n\
                 💡 OPTIMIZATION: [suggestions to reduce cost]",
            )
            .with_timeout(180),
    )
    .add_step(WorkflowStep::new(
        "format-report",
        "fidus",
        StepType::Transform {
            template: "## 💰 Daily Cost Report\n\n\
                {{input}}\n\n\
                ---\n\
                🤖 Generated by Fidus (Ops Agent) | Auto-sent daily at 23:00"
                .to_string(),
        },
    ))
}

/// Optimo Funnel Audit — weekly conversion funnel analysis.
///
/// Runs weekly Monday 9AM (cron `0 9 * * 1`). Agent: Optimo.
/// Output: Funnel metrics, drop-off analysis, A/B test recommendations.
pub fn optimo_funnel_audit() -> Workflow {
    Workflow::new(
        "optimo_funnel_audit",
        "Weekly Funnel Audit — Conversion metrics · Drop-off analysis · A/B test suggestions",
    )
    .with_tags(vec![
        "optimizer",
        "funnel",
        "conversion",
        "ab_test",
        "optimo",
        "agent_team",
    ])
    .with_timeout(900)
    .add_step(
        WorkflowStep::new("analyze-funnel", "optimo", StepType::Sequential)
            .with_prompt(
                "Perform the weekly conversion funnel audit.\n\n\
                 Context: {{input}}\n\n\
                 Analyze each stage:\n\
                 1. **Visit → Signup**: Landing page conversion rate\n\
                 2. **Signup → Trial**: Activation rate\n\
                 3. **Trial → Paid**: Trial-to-paid conversion\n\
                 4. **Overall**: End-to-end conversion\n\n\
                 For each stage:\n\
                 - Current rate vs target vs last week\n\
                 - Trend direction (↑↓→)\n\
                 - Drop-off volume (how many users lost)",
            )
            .with_timeout(300),
    )
    .add_step(
        WorkflowStep::new("diagnose-dropoff", "optimo", StepType::Sequential)
            .with_prompt(
                "Diagnose the biggest conversion drop-offs:\n\n\
                 {{input}}\n\n\
                 For the biggest drop-off stage:\n\
                 1. **Root cause hypothesis** (3 possible reasons)\n\
                 2. **Supporting evidence** (data points, user behavior)\n\
                 3. **Quick fixes** (implement this week)\n\
                 4. **A/B test proposal**:\n\
                    - Hypothesis: Changing X will improve Y because Z\n\
                    - Primary metric to measure\n\
                    - Minimum sample size for significance\n\
                    - Expected duration (minimum 7 days)\n\n\
                 ⚠️ RULES:\n\
                 - Only 1 A/B test at a time\n\
                 - Minimum 7 days before any decision\n\
                 - 95% confidence required to declare winner",
            )
            .with_timeout(300),
    )
    .add_step(
        WorkflowStep::new("active-tests", "optimo", StepType::Sequential)
            .with_prompt(
                "Report on any currently running A/B tests:\n\n\
                 {{input}}\n\n\
                 For each active test:\n\
                 🧪 A/B TEST STATUS — [Test Name]\n\
                 Status: Running (Day [X]/[min 7])\n\
                 Control: [X]% conversion (n=[X])\n\
                 Variant: [X]% conversion (n=[X])\n\
                 Confidence: [X]% (need 95%)\n\
                 Estimated completion: [Date]\n\
                 Decision: WAIT / WINNER / LOSER / INCONCLUSIVE\n\n\
                 If no tests running, state that clearly and reference the new proposal.",
            )
            .with_timeout(180),
    )
    .add_step(WorkflowStep::new(
        "funnel-report",
        "optimo",
        StepType::Transform {
            template: "## 🧪 Weekly Funnel Audit\n\n\
                {{input}}\n\n\
                ---\n\
                📊 Rules: 1 test at a time | Min 7 days | 95% confidence\n\
                🤖 Generated by Optimo (Optimizer Agent) | Weekly Monday 9AM"
                .to_string(),
        },
    ))
}

/// Mercury Outreach — research prospects → draft cold email → send via SES.
///
/// Agent: Mercury (GPT-5 Mini). Scheduled daily 10AM, max 20 emails/day.
/// Guard rails: <100 words, 90s cooldown, opt-out check, positive reply escalation.
pub fn mercury_outreach() -> Workflow {
    Workflow::new(
        "mercury_outreach",
        "Cold Outreach — Prospect Research → Draft Email → Opt-out Check → Send via SES",
    )
    .with_tags(vec![
        "sales",
        "outreach",
        "email",
        "mercury",
        "agent_team",
        "cold_email",
    ])
    .with_timeout(900)
    .add_step(
        WorkflowStep::new("research-prospects", "mercury", StepType::Sequential)
            .with_prompt(
                "Research 5 new DTC/ecommerce founder prospects.\n\n\
                 Context: {{input}}\n\n\
                 Ideal Customer Profile:\n\
                 - DTC or ecommerce brand\n\
                 - Team size: 2-50 people\n\
                 - Revenue: $100K - $10M ARR\n\
                 - Tech-savvy founder/co-founder\n\
                 - Active on Twitter/LinkedIn/IndieHackers\n\n\
                 For each prospect, provide:\n\
                 👤 [Name] — [Company] ([One-line description])\n\
                 Role: [Title] | Size: ~[X] employees | Revenue: ~$[X]\n\
                 Pain point: [Specific problem our product solves]\n\
                 Hook: [Personal detail for email personalization]\n\
                 Score: [1-10 fit score]\n\
                 Email: [If publicly available]\n\n\
                 Only include prospects with score ≥ 7.",
            )
            .with_timeout(300)
            .with_retries(1),
    )
    .add_step(
        WorkflowStep::new("draft-emails", "mercury", StepType::Sequential)
            .with_prompt(
                "Draft personalized cold emails for each prospect:\n\n\
                 {{input}}\n\n\
                 ⛔ HARD RULES:\n\
                 - Each email MUST be under 100 words\n\
                 - Personalized subject line (< 50 chars)\n\
                 - 1 sentence personal hook\n\
                 - 1-2 sentences value prop\n\
                 - 1 soft CTA (question, not demand)\n\
                 - Include unsubscribe link placeholder\n\
                 - American English, professional tone\n\
                 - NO generic greetings (Dear Sir/Madam)\n\
                 - NO multiple CTAs\n\
                 - NO aggressive sales language\n\n\
                 Format each email clearly with SUBJECT and BODY separated.",
            )
            .with_timeout(300),
    )
    .add_step(
        WorkflowStep::new("optout-check", "mercury", StepType::Sequential)
            .with_prompt(
                "Verify opt-out compliance for these emails:\n\n\
                 {{input}}\n\n\
                 Check each recipient against:\n\
                 1. Opt-out list (data/agent-team/optout.json)\n\
                 2. 30-day no-repeat rule (check sent_log)\n\
                 3. CAN-SPAM Act compliance\n\
                 4. Domain blacklist\n\n\
                 For each email, mark:\n\
                 ✅ CLEAR — safe to send\n\
                 ❌ BLOCKED — [reason]\n\n\
                 Only CLEAR emails proceed to sending.",
            )
            .with_timeout(120),
    )
    .add_step(WorkflowStep::new(
        "send-report",
        "mercury",
        StepType::Transform {
            template: "## 📧 Outreach Batch Ready\n\n\
                {{input}}\n\n\
                ---\n\
                ⛔ Limits: <20 emails/day | <100 words | 90s cooldown\n\
                📋 CAN-SPAM compliant | Opt-out checked\n\
                🤖 Generated by Mercury (Sales Agent)\n\
                ⚠️ Positive replies will be escalated to Max immediately"
                .to_string(),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_workflows_count() {
        let workflows = builtin_workflows();
        assert_eq!(workflows.len(), 23);
    }

    #[test]
    fn test_content_pipeline_structure() {
        let wf = content_pipeline();
        assert_eq!(wf.name, "content_pipeline");
        assert_eq!(wf.step_count(), 3);
        assert!(wf.get_step("draft").is_some());
        assert!(wf.get_step("review").is_some());
        assert!(wf.get_step("edit").is_some());
    }

    #[test]
    fn test_expert_consensus_structure() {
        let wf = expert_consensus();
        assert_eq!(wf.name, "expert_consensus");
        assert!(wf.step_count() >= 4);
    }

    #[test]
    fn test_code_review_has_optional() {
        let wf = code_review_pipeline();
        let style_step = wf.get_step("style").unwrap();
        assert!(style_step.optional);
    }

    #[test]
    fn test_all_workflows_have_tags() {
        for wf in builtin_workflows() {
            assert!(!wf.tags.is_empty(), "Workflow '{}' has no tags", wf.name);
        }
    }

    #[test]
    fn test_slide_creator_structure() {
        let wf = slide_creator();
        assert_eq!(wf.name, "slide_creator");
        assert!(wf.step_count() >= 10);
        assert!(wf.get_step("research").is_some());
        assert!(wf.get_step("parallel-gen").is_some());
        assert!(wf.get_step("export").is_some());
    }

    #[test]
    fn test_meeting_recap_structure() {
        let wf = meeting_recap();
        assert_eq!(wf.name, "meeting_recap");
        assert!(wf.step_count() >= 4);
        assert!(wf.get_step("summarize").is_some());
        assert!(wf.get_step("extract-tasks").is_some());
    }

    #[test]
    fn test_ceo_briefing() {
        let wf = ceo_daily_briefing();
        assert_eq!(wf.name, "ceo_daily_briefing");
        assert!(wf.step_count() >= 6);
        assert!(wf.get_step("parallel-gather").is_some());
    }

    #[test]
    fn test_competitor_analysis() {
        let wf = competitor_analysis();
        assert!(wf.step_count() >= 4);
        assert!(wf.get_step("compare-swot").is_some());
    }

    #[test]
    fn test_proposal_generator() {
        let wf = proposal_generator();
        assert!(wf.step_count() >= 4);
        assert!(wf.get_step("draft-proposal").is_some());
    }

    #[test]
    fn test_weekly_report() {
        let wf = weekly_report();
        assert!(wf.step_count() >= 7);
        assert!(wf.get_step("executive-summary").is_some());
    }

    #[test]
    fn test_email_drip_campaign() {
        let wf = email_drip_campaign();
        assert_eq!(wf.name, "email_drip_campaign");
        assert!(wf.step_count() >= 9);
        assert!(wf.get_step("email-1").is_some());
        assert!(wf.get_step("email-5").is_some());
        assert!(wf.get_step("parallel-emails").is_some());
    }

    #[test]
    fn test_hiring_pipeline() {
        let wf = hiring_pipeline();
        assert_eq!(wf.name, "hiring_pipeline");
        assert!(wf.step_count() >= 4);
        assert!(wf.get_step("job-description").is_some());
        assert!(wf.get_step("interview-questions").is_some());
        assert!(wf.get_step("eval-matrix").is_some());
    }

    #[test]
    fn test_customer_feedback() {
        let wf = customer_feedback_analysis();
        assert_eq!(wf.name, "customer_feedback");
        assert!(wf.step_count() >= 4);
        assert!(wf.get_step("categorize").is_some());
        assert!(wf.get_step("action-plan").is_some());
    }

    #[test]
    fn test_contract_review() {
        let wf = contract_review();
        assert_eq!(wf.name, "contract_review");
        assert!(wf.step_count() >= 4);
        assert!(wf.get_step("risk-assess").is_some());
    }

    #[test]
    fn test_product_launch() {
        let wf = product_launch_checklist();
        assert_eq!(wf.name, "product_launch");
        assert!(wf.step_count() >= 7);
        assert!(wf.get_step("content-pack").is_some());
        assert!(wf.get_step("pr-draft").is_some());
    }
}
