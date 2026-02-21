---
description: 🚀 Deploy lên Production - The Release Manager
---

# WORKFLOW: /deploy - Complete Production Deployment Guide

> **Role**: Antigravity DevOps Engineer  
> **Mission**: Đưa app lên Internet với đầy đủ checklist production
> **Principle**: "User không biết cần những gì, AI phải chủ động"

// turbo-all

---

## 🎯 Mục đích

Workflow này đảm bảo deployment có:
- Pre-flight checks (build, security)
- SEO optimization
- Analytics tracking
- Legal compliance (GDPR/PDPA)
- Backup strategy
- Monitoring & alerting
- SSL/HTTPS configuration

---

## Phase 1: Deployment Discovery 🎯

### 1.1 Purpose Check

```markdown
"Deploy để làm gì?"

┌─────────────────────────────────────────────────┐
│ A) 👁️ Preview (Chỉ mình anh xem)                │
│    → Vercel Preview, Netlify Draft             │
│                                                  │
│ B) 🧪 Staging (Cho team test)                   │
│    → Staging environment, basic setup           │
│                                                  │
│ C) 🚀 Production (Khách hàng dùng)              │
│    → Full production checklist                   │
└─────────────────────────────────────────────────┘
```

### 1.2 Domain Check

```markdown
"Có tên miền chưa?"

Chưa có:
  → Gợi ý mua: Namecheap, Cloudflare, Google Domains
  → Free subdomain: *.vercel.app, *.netlify.app, *.railway.app

Có rồi:
  → "Domain là gì?"
  → "Có access DNS settings không?"
```

### 1.3 Hosting Options

```markdown
"Có server riêng không?"

Không có (Recommend):
┌─────────────────────────────────────────────────┐
│ Frontend Only:                                   │
│ - Vercel (Best for Next.js)                     │
│ - Netlify (Great for static)                    │
│ - Cloudflare Pages (Global CDN)                 │
│                                                  │
│ Full Stack:                                      │
│ - Railway (Easy, auto-scaling)                  │
│ - Render (Good free tier)                       │
│ - Fly.io (Edge deployment)                      │
│                                                  │
│ Enterprise:                                      │
│ - AWS (EC2, ECS, Lambda)                        │
│ - GCP (Cloud Run, GKE)                          │
│ - Azure (App Service)                           │
└─────────────────────────────────────────────────┘

Có sẵn:
  → "OS gì?" (Ubuntu/Debian/CentOS)
  → "Có Docker không?"
  → "SSH access?"
```

---

## Phase 2: Pre-Flight Check ✅

### 2.1 Build Verification

```bash
# Clean build
rm -rf node_modules/.cache
rm -rf .next/cache
npm ci
npm run build

# Expected: Build success with no errors
# If failed → STOP, run /debug
```

### 2.2 Environment Check

```markdown
□ .env.production exists and complete?
□ All required variables set?
□ No development/test values?
□ Secrets properly secured?
```

```bash
# Required .env.production variables
DATABASE_URL=postgresql://...
REDIS_URL=redis://...
JWT_SECRET=<32+ char random string>
NEXT_PUBLIC_API_URL=https://api.domain.com

# Validate no localhost references
grep -r "localhost" .env.production  # Should be empty
```

### 2.3 Security Pre-Check

```markdown
□ No hardcoded secrets in code?
□ Debug mode disabled?
□ CORS properly configured?
□ Rate limiting enabled?
□ Input validation complete?
```

```bash
# Quick security scan
grep -r "API_KEY\|SECRET\|PASSWORD" src/  # Check for hardcoded secrets
grep -r "console.log\|debugger" src/       # Check for debug code
```

---

## Phase 3: SEO Setup 🔍

> ⚠️ User thường quên hoàn toàn phần này!

### 3.1 Explain to User

```markdown
"Để Google tìm thấy app của anh, cần setup SEO."
"Không có SEO = Invisible trên internet."
```

### 3.2 SEO Checklist

#### Meta Tags (Every Page)
```html
<head>
  <title>Page Title | Brand Name</title>
  <meta name="description" content="Compelling description under 160 chars">
  <meta name="robots" content="index, follow">
  <link rel="canonical" href="https://domain.com/page">
</head>
```

#### Open Graph (Social Sharing)
```html
<meta property="og:title" content="Page Title">
<meta property="og:description" content="Description for social">
<meta property="og:image" content="https://domain.com/og-image.jpg">
<meta property="og:url" content="https://domain.com/page">
<meta property="og:type" content="website">
```

#### Sitemap
```xml
<!-- sitemap.xml -->
<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>https://domain.com/</loc>
    <lastmod>2026-01-16</lastmod>
    <priority>1.0</priority>
  </url>
  <!-- More URLs... -->
</urlset>
```

#### Robots.txt
```
User-agent: *
Allow: /
Disallow: /api/
Disallow: /admin/
Sitemap: https://domain.com/sitemap.xml
```

### 3.3 Auto-Generate

```markdown
AI sẽ tự động:
✅ Generate meta tags template
✅ Create sitemap.xml
✅ Create robots.txt
✅ Setup Open Graph defaults
```

---

## Phase 4: Analytics Setup 📊

> ⚠️ User không biết cần theo dõi traffic!

### 4.1 Explain to User

```markdown
"Anh có muốn biết bao nhiêu người truy cập không?"
"Họ từ đâu đến? Họ làm gì trên app?"

(Answer is always: YES!)
```

### 4.2 Analytics Options

```markdown
┌─────────────────────────────────────────────────┐
│ Free & Popular:                                  │
│ - Google Analytics 4 (Full-featured)            │
│ - Plausible (Privacy-focused, simple)           │
│ - Umami (Self-hosted, privacy)                  │
│                                                  │
│ Advanced:                                        │
│ - Mixpanel (Event tracking)                     │
│ - Amplitude (Product analytics)                 │
│ - PostHog (Open source, full suite)            │
└─────────────────────────────────────────────────┘
```

### 4.3 Implementation

```typescript
// Google Analytics 4
import { GoogleAnalytics } from '@next/third-parties/google';

export default function RootLayout({ children }) {
  return (
    <html>
      <body>
        {children}
        <GoogleAnalytics gaId="G-XXXXXXXXXX" />
      </body>
    </html>
  );
}
```

---

## Phase 5: Legal Compliance 📋

> ⚠️ Bắt buộc theo luật GDPR/PDPA!

### 5.1 Required Pages

```markdown
AI sẽ tạo templates cho:

1. Privacy Policy
   - Data collection practices
   - Third-party services used
   - User rights
   - Contact information

2. Terms of Service
   - Usage terms
   - Liability limitations
   - Account termination
   - Dispute resolution

3. Cookie Consent Banner
   - "We use cookies..."
   - Accept/Decline options
   - Link to Privacy Policy
```

### 5.2 Cookie Consent Implementation

```typescript
// Simple cookie consent banner
import CookieConsent from 'react-cookie-consent';

<CookieConsent
  location="bottom"
  buttonText="Đồng ý"
  declineButtonText="Từ chối"
  enableDeclineButton
  onAccept={() => enableAnalytics()}
  onDecline={() => disableAnalytics()}
>
  Website sử dụng cookies để cải thiện trải nghiệm. 
  <a href="/privacy">Xem Privacy Policy</a>
</CookieConsent>
```

---

## Phase 6: Backup Strategy 💾

> ⚠️ User không nghĩ tới cho đến khi mất data!

### 6.1 Explain to User

```markdown
"Nếu server chết hoặc bị hack, anh có muốn mất hết dữ liệu không?"
"Em sẽ setup backup tự động."
```

### 6.2 Backup Plan

```markdown
┌─────────────────────────────────────────────────┐
│ Database Backup:                                 │
│ - Daily automated backup                        │
│ - Retain last 7 days                           │
│ - Weekly backup → retain 4 weeks               │
│ - Monthly backup → retain 12 months            │
│                                                  │
│ File/Upload Backup:                             │
│ - Sync to cloud storage (S3/GCS)               │
│ - Versioning enabled                           │
│                                                  │
│ Code Backup:                                    │
│ - Git (already done ✅)                         │
└─────────────────────────────────────────────────┘
```

### 6.3 Implementation

```bash
# PostgreSQL backup cron job
# Add to crontab: crontab -e
0 3 * * * pg_dump $DATABASE_URL | gzip > /backups/db_$(date +\%Y\%m\%d).sql.gz

# Delete backups older than 7 days
0 4 * * * find /backups -name "*.sql.gz" -mtime +7 -delete
```

---

## Phase 7: Monitoring & Alerting 📡

> ⚠️ User không biết app chết lúc 3AM!

### 7.1 Setup Components

```markdown
┌─────────────────────────────────────────────────┐
│ Uptime Monitoring:                               │
│ - UptimeRobot (Free, checks every 5 min)        │
│ - Better Uptime (Pretty status page)            │
│ - Pingdom (Enterprise)                          │
│                                                  │
│ Error Tracking:                                  │
│ - Sentry (Best for frontend+backend)            │
│ - LogRocket (Session replay)                    │
│ - Bugsnag (Alternative)                         │
│                                                  │
│ Log Management:                                  │
│ - Logtail (Simple, affordable)                  │
│ - Papertrail (Traditional)                      │
│ - Grafana Loki (Self-hosted)                    │
└─────────────────────────────────────────────────┘
```

### 7.2 Sentry Integration

```typescript
// sentry.client.config.ts
import * as Sentry from '@sentry/nextjs';

Sentry.init({
  dsn: process.env.SENTRY_DSN,
  environment: process.env.NODE_ENV,
  tracesSampleRate: 0.1, // 10% of transactions
  replaysSessionSampleRate: 0.1,
});
```

---

## Phase 8: Deployment Execution 🚀

### 8.1 SSL/HTTPS

```markdown
Automatic SSL với:
- Cloudflare (Recommended, free)
- Let's Encrypt (Free, auto-renew)
- Vercel/Netlify (Built-in)
```

### 8.2 DNS Configuration

```markdown
Hướng dẫn step-by-step:

1. Login vào DNS provider (Cloudflare/Namecheap/...)
2. Add records:
   - A record: @ → [Server IP]
   - CNAME: www → domain.com
   - (If Vercel): CNAME: @ → cname.vercel-dns.com
3. Wait 5-30 minutes for propagation
4. Verify: nslookup domain.com
```

### 8.3 Deploy Commands

```bash
# Vercel
vercel --prod

# Railway
railway up

# Docker
docker build -t app:latest .
docker push registry/app:latest
kubectl apply -f k8s/

# Traditional VPS
ssh user@server
cd /app
git pull
npm ci
npm run build
pm2 restart app
```

---

## Phase 9: Post-Deploy Verification ✅

```markdown
Checklist sau khi deploy:

□ Homepage loads?
□ Login/signup works?
□ Core features functional?
□ Mobile responsive?
□ SSL certificate valid?
□ Analytics tracking?
□ Error tracking active?
□ Uptime monitoring active?
□ Backups scheduled?
□ DNS propagated?
```

---

## Phase 10: Handover 🎉

```markdown
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🚀 DEPLOYMENT COMPLETE!
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

🌐 URL: https://yourdomain.com

Checklist hoàn thành:
✅ App online và chạy ổn định
✅ SSL/HTTPS enabled
✅ SEO optimized
✅ Analytics tracking active
✅ Legal pages created
✅ Backup scheduled daily
✅ Monitoring active (Sentry + UptimeRobot)

📊 Health Check:
- Homepage: 200 OK (245ms)
- API: 200 OK (89ms)
- Database: Connected
- SSL: Valid until 2027-01-16

⚠️ Nhớ: /save-brain để lưu cấu hình deployment!
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

---

## ⚠️ NEXT STEPS

```
1️⃣ Deploy OK? → /save-brain để lưu config
2️⃣ Có lỗi? → /debug để sửa
3️⃣ Cần rollback? → /rollback
4️⃣ Monitor đang hoạt động? → Check Sentry/UptimeRobot
```

---

## 💡 DEPLOYMENT TIPS

| Tip | Description |
|-----|-------------|
| Feature flags | Deploy code, enable later |
| Blue-green | Zero downtime deployment |
| Canary | Roll out to 10% users first |
| Rollback plan | Always know how to revert |
| Health checks | Verify before traffic switch |

---

*"Deploy early, deploy often, but always with a plan."*
