---
description: 🎨 Thiết kế giao diện - The Creative Partner
---

# WORKFLOW: /visualize - Comprehensive UI/UX Design System

> **Role**: Antigravity Creative Director  
> **Mission**: Biến "Vibe" thành giao diện đẹp, dễ dùng, chuyên nghiệp
> **Principle**: "User có Gu, AI có Skills"

// turbo-all

---

## 🎯 Mục đích

Workflow này giúp:
- Chuyển đổi ý tưởng mơ hồ thành design cụ thể
- Phát hiện yêu cầu UX mà user chưa nghĩ tới
- Tạo mockup và implement pixel-perfect
- Đảm bảo accessibility và responsive

---

## Phase 1: Vibe Styling 🎭

### 1.1 Design Direction

```markdown
"Anh muốn giao diện nhìn nó thế nào?"

┌─────────────────────────────────────────────────┐
│ A) ☀️ Sáng sủa, sạch sẽ (Clean, Minimal)        │
│    → Apple, Stripe, Linear                       │
│                                                  │
│ B) 🌙 Sang trọng, cao cấp (Luxury, Dark)        │
│    → Vercel, Discord, Notion Dark               │
│                                                  │
│ C) 🌈 Trẻ trung, năng động (Colorful, Playful)  │
│    → Figma, Slack, Canva                        │
│                                                  │
│ D) 💼 Chuyên nghiệp, doanh nghiệp (Corporate)   │
│    → Salesforce, Microsoft, SAP                  │
│                                                  │
│ E) 🚀 Công nghệ, hiện đại (Tech, Futuristic)    │
│    → Framer, Raycast, Arc Browser               │
└─────────────────────────────────────────────────┘
```

### 1.2 Color Palette

```markdown
"Có màu chủ đạo nào anh thích không?"

Options:
- 🎨 Theo màu Logo công ty
- 🔵 Blue (Trust, Corporate)
- 💜 Purple (Creative, Premium)
- 🟢 Green (Growth, Finance)
- 🔴 Red (Energy, Food)
- ⚫ Neutral (Modern, Tech)

"Anh thích nền sáng (Light) hay nền tối (Dark)?"
"Cần support cả hai modes không?"
```

### 1.3 Shape & Style

```markdown
"Các góc bo tròn mềm mại hay vuông vức sắc cạnh?"

A) 🔘 Rounded (border-radius: 12px+) → Friendly
B) ⬜ Sharp (border-radius: 0-4px) → Professional  
C) 💎 Mixed → Cards rounded, buttons sharp

"Có cần hiệu ứng bóng đổ (Shadow) không?"
"Có thích style Glassmorphism không?"
```

---

## Phase 2: Hidden UX Discovery 🔍

### 2.1 Device Priority

```markdown
"Người dùng sẽ xem trên thiết bị nào nhiều hơn?"

📱 Mobile First:
- Touch-friendly buttons (min 44px)
- Bottom navigation
- Hamburger menu
- Swipe gestures

💻 Desktop First:
- Sidebar navigation
- Wide data tables
- Keyboard shortcuts
- Hover states
```

### 2.2 Loading States

```markdown
"Khi đang tải dữ liệu, anh muốn hiện gì?"

A) ⭕ Spinner (Vòng xoay) → Simple
B) ━━━ Progress bar → Có tiến độ rõ ràng
C) 💀 Skeleton → Chuyên nghiệp nhất, giảm CLS
D) 🎯 Custom animation → Brand identity
```

### 2.3 Empty States

```markdown
"Khi chưa có dữ liệu, hiện gì?"

AI sẽ tự thiết kế:
- Illustration thân thiện
- Message hướng dẫn clear
- CTA button để thêm data
- Ví dụ: "Chưa có đơn hàng nào. Tạo đơn đầu tiên?"
```

### 2.4 Error States

```markdown
"Khi có lỗi xảy ra, báo kiểu nào?"

A) 🔔 Modal popup → Lỗi nghiêm trọng
B) 📢 Banner top → Lỗi cần attention
C) 🍞 Toast corner → Thông báo nhanh
D) 📝 Inline → Form validation
```

### 2.5 Accessibility ♿

```markdown
AI TỰ ĐỘNG đảm bảo:
✅ Color contrast ratio ≥ 4.5:1 (WCAG AA)
✅ Focus indicators visible
✅ Alt text cho images
✅ Keyboard navigation
✅ Screen reader labels
✅ Skip to content link
✅ Proper heading hierarchy
```

### 2.6 Dark Mode

```markdown
"Có cần chế độ tối (Dark mode) không?"

Nếu CÓ:
- Design cả 2 phiên bản
- CSS variables cho theming
- System preference detection
- Manual toggle option
```

---

## Phase 3: Reference & Inspiration 🖼️

### 3.1 Gather Inspiration

```markdown
"Có website/app nào anh thấy đẹp muốn tham khảo?"

Nếu CÓ → Analyze and extract:
- Color palette
- Typography
- Spacing system
- Component patterns

Nếu KHÔNG → AI suggest based on industry:
- E-commerce → Shopify, Amazon
- SaaS → Linear, Notion
- Finance → Stripe, Wise
- Social → Twitter, Instagram
```

### 3.2 Design System Foundation

```css
/* Generated Design Tokens */
:root {
  /* Colors */
  --color-primary: #6366f1;
  --color-primary-hover: #4f46e5;
  --color-secondary: #f1f5f9;
  --color-success: #10b981;
  --color-warning: #f59e0b;
  --color-error: #ef4444;
  
  /* Typography */
  --font-sans: 'Inter', system-ui, sans-serif;
  --font-mono: 'JetBrains Mono', monospace;
  
  /* Spacing */
  --space-1: 0.25rem;
  --space-2: 0.5rem;
  --space-4: 1rem;
  --space-8: 2rem;
  
  /* Shadows */
  --shadow-sm: 0 1px 2px rgba(0,0,0,0.05);
  --shadow-md: 0 4px 6px rgba(0,0,0,0.1);
  --shadow-lg: 0 10px 15px rgba(0,0,0,0.1);
  
  /* Radii */
  --radius-sm: 0.25rem;
  --radius-md: 0.5rem;
  --radius-lg: 1rem;
  --radius-full: 9999px;
}
```

---

## Phase 4: Mockup Generation 🎨

### 4.1 Create Mockup

```markdown
Prompt cho generate_image:

"Modern web dashboard UI design, [style từ Phase 1],
primary color [hex], dark/light mode,
showing [component type],
clean layout, professional,
[specific elements]"
```

### 4.2 Iteration Loop

```markdown
User: "Hơi tối"
→ AI: Tăng brightness, lighten backgrounds

User: "Nhìn tù tù"  
→ AI: Thêm spacing, shadows, gradients

User: "Màu chói quá"
→ AI: Giảm saturation, softer palette

User: "Thiếu chiều sâu"
→ AI: Add layering, shadows, borders
```

---

## Phase 5: Pixel-Perfect Implementation 💻

### 5.1 Component Breakdown

```markdown
Phân tích mockup thành components:

📦 Layout
├── Header (Logo, Nav, User menu)
├── Sidebar (Navigation, collapse)
├── Main Content (Grid, Cards)
└── Footer (Links, Copyright)

📦 Components
├── Button (Primary, Secondary, Ghost)
├── Card (Basic, Interactive, Stat)
├── Form (Input, Select, Checkbox)
├── Table (Sortable, Filterable)
└── Modal (Confirm, Form, Alert)
```

### 5.2 Responsive Implementation

```css
/* Mobile First Breakpoints */
/* Base: Mobile (< 640px) */

@media (min-width: 640px) {
  /* sm: Tablet Portrait */
}

@media (min-width: 768px) {
  /* md: Tablet Landscape */
}

@media (min-width: 1024px) {
  /* lg: Desktop */
}

@media (min-width: 1280px) {
  /* xl: Large Desktop */
}
```

### 5.3 Animation & Micro-interactions

```css
/* Smooth transitions */
.button {
  transition: all 0.2s ease-out;
}

.button:hover {
  transform: translateY(-1px);
  box-shadow: var(--shadow-md);
}

.button:active {
  transform: translateY(0);
}

/* Page transitions */
.page-enter {
  opacity: 0;
  transform: translateY(10px);
}

.page-enter-active {
  opacity: 1;
  transform: translateY(0);
  transition: all 0.3s ease-out;
}
```

### 5.4 State Coverage

✅ Default state
✅ Hover state  
✅ Focus state
✅ Active/Pressed state
✅ Disabled state
✅ Loading state
✅ Error state
✅ Success state
✅ Empty state

---

## Phase 6: Quality Assurance 🔍

### 6.1 Accessibility Check

```bash
# Lighthouse Accessibility Score Target: 90+
lighthouse --only-categories=accessibility

# Manual checks:
□ Tab navigation works
□ Focus visible
□ Color contrast OK
□ Screen reader tested
□ Keyboard shortcuts work
```

### 6.2 Cross-Browser Testing

```markdown
□ Chrome (latest)
□ Firefox (latest)
□ Safari (latest)
□ Edge (latest)
□ Mobile Safari (iOS)
□ Chrome Mobile (Android)
```

### 6.3 Performance Check

```markdown
□ No layout shift (CLS < 0.1)
□ First paint < 1.5s
□ Images optimized
□ Fonts properly loaded
□ CSS minified
```

---

## Phase 7: Handover ✅

```markdown
"Giao diện đã xong."

Checklist:
✅ Desktop layout hoàn chỉnh
✅ Mobile responsive
✅ Dark mode (nếu yêu cầu)
✅ All states implemented
✅ Accessibility compliant
✅ Animations smooth

"Anh xem thử:"
- Desktop: http://localhost:3000
- Mobile: Dùng DevTools (F12 → Toggle device)
```

---

## ⚠️ NEXT STEPS

```
1️⃣ UI OK? → /code để thêm logic
2️⃣ Cần chỉnh sửa UI? → Tiếp tục trong /visualize
3️⃣ Lỗi hiển thị? → /debug
4️⃣ Ready deploy? → /deploy
5️⃣ Lưu thiết kế? → /save-brain
```

---

## 💡 DESIGN RULES

| Rule | Description |
|------|-------------|
| Consistency | Dùng design tokens, không hard-code |
| Hierarchy | Rõ ràng visual priority |
| Whitespace | Đừng sợ khoảng trống |
| Feedback | Mọi action đều có response |
| Simplicity | Khi nghi ngờ, bỏ bớt đi |

---

*"Good design is invisible. Great design is memorable."*
