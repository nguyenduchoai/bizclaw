# BizClaw Accessibility & Performance Improvement Roadmap

**Ngày bắt đầu**: 2026-04-16  
**Phiên bản**: 1.1.7  
**Thời gian**: 4 tuần  
**Mục tiêu**: WCAG 2.1 AA Compliance + Core Web Vitals Optimization

---

## TÓM TẮT ĐIỂM HIỆN TẠI

| Hạng mục | Điểm hiện tại | Mục tiêu | Gap |
|-----------|---------------|-----------|-----|
| **Accessibility Score** | 72/100 | 95/100 | +23 |
| **Performance Score** | 85/100 | 95/100 | +10 |
| **Lighthouse Performance** | 85 | 95 | +10 |
| **First Contentful Paint** | 2.1s | <1.5s | -0.6s |
| **Largest Contentful Paint** | 3.2s | <2.5s | -0.7s |
| **Cumulative Layout Shift** | 0.15 | <0.1 | -0.05 |
| **Color Contrast Issues** | 5 | 0 | -5 |

---

## WEEK 1: Critical Bug Fixes & Basic Accessibility Foundation

### 📅 Ngày 1-2: Bug Analysis & Prioritization

```
┌─────────────────────────────────────────────────────────────────┐
│ DAY 1-2: BUG ANALYSIS                                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│ P0 Critical Bugs (Must Fix):                                    │
│ ───────────────────────────────────                            │
│ 1. BUG-01: Sidebar backdrop not showing on mobile             │
│    • Issue: z-index conflict, display:none override            │
│    • Impact: User can't close mobile menu                      │
│    • Fix: Adjust z-index hierarchy (99→1001)                   │
│                                                                 │
│ 2. BUG-02: Toast notifications stacking incorrectly             │
│    • Issue: Multiple toasts overlap, no z-index management     │
│    • Impact: Error messages hidden                             │
│    • Fix: Implement toast queue with position management        │
│                                                                 │
│ 3. BUG-03: Date picker not accessible on mobile                 │
│    • Issue: Native date picker styling inconsistent             │
│    • Impact: Screen reader can't navigate                       │
│    • Fix: Custom accessible date picker component               │
│                                                                 │
│ P1 Major Bugs (Should Fix):                                     │
│ ─────────────────────────────────                              │
│ 4. BUG-04: Active nav state not updating on route change       │
│ 5. BUG-05: Multi-select dropdown z-index issue                 │
│ 6. BUG-06: Table horizontal scroll on mobile                   │
│ 7. BUG-07: Images not lazy-loaded                              │
│ 8. BUG-08: Button double-click not prevented                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 📅 Ngày 3-4: Core Accessibility Implementation

```javascript
// =============================================================
// ACCESSIBILITY FEATURES TO IMPLEMENT
// =============================================================

// 1. SKIP TO CONTENT LINK
// File: dashboard/index.html, dashboard-new.html
<a href="#main-content" class="skip-link">
  Skip to main content
</a>

<style>
.skip-link {
  position: absolute;
  top: -40px;
  left: 0;
  background: var(--accent);
  color: white;
  padding: 8px;
  z-index: 10000;
  transition: top 0.3s;
}
.skip-link:focus {
  top: 0;
}
</style>

// 2. FOCUS INDICATORS
// File: styles.css, dashboard-new.css
*:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}

// 3. ARIA LABELS FOR NAVIGATION
// File: shared.js (navigation rendering)
function renderNavItem(item) {
  return `
    <a href="${item.href}" 
       role="menuitem"
       aria-label="${item.description || item.label}"
       aria-current="${item.active ? 'page' : 'false'}">
      <span aria-hidden="true">${item.icon}</span>
      <span>${item.label}</span>
    </a>
  `;
}

// 4. LIVE REGIONS FOR DYNAMIC CONTENT
// File: shared.js (toast notifications)
<div id="toast-container" 
     role="status" 
     aria-live="polite" 
     aria-atomic="true">
</div>

// 5. SEMANTIC HTML STRUCTURE
// File: dashboard/index.html
<header role="banner">
  <nav role="navigation" aria-label="Main navigation">
  </nav>
</header>
<main id="main-content" role="main">
</main>
<aside role="complementary">
</aside>
<footer role="contentinfo">
</footer>
```

### 📅 Ngày 5: Accessibility Audit

```bash
# =============================================================
# ACCESSIBILITY TESTING COMMANDS
# =============================================================

# 1. Install axe-core for automated testing
npm install @axe-core/playwright

# 2. Run accessibility audit with Playwright
npx playwright test --project=chromium --headed
# Check console for axe violations

# 3. Manual Screen Reader Testing
# - NVDA + Chrome (Windows)
# - VoiceOver + Safari (macOS)
# - TalkBack + Chrome (Android)

# 4. Keyboard Navigation Audit
# Tab through all interactive elements
# Verify focus order is logical
# Check all functions accessible via keyboard

# 5. Color Contrast Check
# Use browser DevTools > Elements > Color Picker
# Verify contrast ratios meet WCAG AA
```

### 📅 Ngày 6-7: Bug Fix Implementation

```javascript
// =============================================================
// BUG-01 FIX: Sidebar Backdrop
// File: shared.js
function toggleMobileSidebar(open) {
  const sidebar = document.querySelector('.sidebar');
  const backdrop = document.querySelector('.sidebar-backdrop');
  
  if (open) {
    sidebar.classList.add('open');
    backdrop.style.display = 'block';
    backdrop.classList.add('show');
    backdrop.setAttribute('aria-hidden', 'false');
    // Trap focus within sidebar
    trapFocus(sidebar);
  } else {
    sidebar.classList.remove('open');
    backdrop.classList.remove('show');
    backdrop.setAttribute('aria-hidden', 'true');
    setTimeout(() => {
      backdrop.style.display = 'none';
    }, 300);
  }
}

// =============================================================
// BUG-02 FIX: Toast Queue Management
// File: shared.js
class ToastManager {
  constructor() {
    this.queue = [];
    this.maxVisible = 3;
  }
  
  show(message, type = 'info', duration = 5000) {
    const toast = this.createToast(message, type);
    this.queue.push(toast);
    this.render();
    
    setTimeout(() => this.dismiss(toast.id), duration);
  }
  
  render() {
    const container = document.getElementById('toast-container');
    container.innerHTML = this.queue
      .slice(0, this.maxVisible)
      .map(t => t.element)
      .join('');
  }
  
  dismiss(id) {
    this.queue = this.queue.filter(t => t.id !== id);
    this.render();
  }
}
```

### ✅ Week 1 Deliverables

| Deliverable | File | Status |
|-------------|------|--------|
| Bug Fix Documentation | `/docs/week1-bugfix-log.md` | ⬜ |
| Resolved Issue Tracker | `/docs/week1-issues-resolved.md` | ⬜ |
| Accessibility Checklist | `/docs/week1-a11y-checklist.md` | ⬜ |
| axe-core Audit Report | `/docs/week1-axe-report.md` | ⬜ |

---

## WEEK 2: Performance Optimization

### 📅 Ngày 1-2: Performance Audit & Analysis

```
┌─────────────────────────────────────────────────────────────────┐
│ PERFORMANCE ANALYSIS                                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│ Current Bundle Analysis:                                         │
│ ─────────────────────                                          │
│  • styles.css:        25 KB (6 KB gzipped)                      │
│  • dashboard-new.css: 20 KB (5 KB gzipped)                     │
│  • app.js:           150 KB (45 KB gzipped)                    │
│  • app-new.js:       180 KB (50 KB gzipped)                    │
│  • shared.js:        50 KB (15 KB gzipped)                    │
│  • Total:            425 KB (121 KB gzipped)                    │
│                                                                 │
│ Load Time by Connection:                                         │
│ ──────────────────────────                                      │
│  3G (1.6 Mbps):     3.5s → Target: 2.0s                        │
│  4G (10 Mbps):      1.5s → Target: 0.8s                         │
│  Broadband (50 Mbps): 0.5s → Target: 0.3s                       │
│                                                                 │
│ Bottlenecks Identified:                                          │
│ ─────────────────────                                           │
│  1. No code splitting (all JS loaded on initial page)          │
│  2. Images not lazy-loaded                                       │
│  3. No service worker caching                                   │
│  4. CSS not minified (debug mode)                               │
│  5. Font files not subsetted                                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 📅 Ngày 3-4: Image Optimization & Lazy Loading

```javascript
// =============================================================
// IMAGE LAZY LOADING
// File: shared.js

// Add lazy loading attribute to images
function optimizeImages() {
  // Native lazy loading
  document.querySelectorAll('img').forEach(img => {
    if (!img.hasAttribute('loading')) {
      img.setAttribute('loading', 'lazy');
    }
  });
  
  // Intersection Observer for background images
  const observer = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        const img = entry.target;
        if (img.dataset.src) {
          img.src = img.dataset.src;
          img.removeAttribute('data-src');
          observer.unobserve(img);
        }
      }
    });
  }, { rootMargin: '50px' });
  
  document.querySelectorAll('[data-src]').forEach(img => {
    observer.observe(img);
  });
}

// =============================================================
// IMAGE COMPRESSION CONFIG
// File: webpack.config.js or vite.config.js

export default {
  build: {
    rollupOptions: {
      output: {
        assetFileNames: '[name].[hash][extname]',
        manualChunks: {
          vendor: ['preact', 'htm'],
        }
      }
    },
    // Image optimization
    assetsInlineLimit: 4096, // 4kb
    // Minification
    minify: 'terser',
    terserOptions: {
      compress: {
        drop_console: true,
        drop_debugger: true
      }
    }
  }
};
```

### 📅 Ngày 5: Code Splitting & Bundling

```javascript
// =============================================================
// DYNAMIC IMPORT FOR CODE SPLITTING
// File: app.js

// Page-based code splitting
const routes = {
  'chat': () => import('./pages/chat.js'),
  'dashboard': () => import('./pages/dashboard.js'),
  'settings': () => import('./pages/settings.js'),
  'channels': () => import('./pages/channels.js'),
  'agents': () => import('./pages/agents.js'),
  'crm': () => import('./pages/crm.js'),
  'knowledge': () => import('./pages/knowledge.js'),
  'workflows': () => import('./pages/workflows.js'),
};

async function loadPage(pageName) {
  if (routes[pageName]) {
    showLoadingSkeleton();
    const module = await routes[pageName]();
    renderPage(module.default);
    hideLoadingSkeleton();
  }
}

// =============================================================
// CSS TREE SHAKING
// File: rollup.config.js

export default {
  build: {
    cssCodeSplit: true,
    rollupOptions: {
      output: {
        manualChunks: (id) => {
          // Split vendor code
          if (id.includes('node_modules')) {
            return 'vendor';
          }
          // Split page-specific CSS
          if (id.includes('/pages/')) {
            return `page-${id.split('/').pop().replace('.js', '')}`;
          }
        }
      }
    }
  }
};
```

### 📅 Ngày 6-7: Caching & Service Worker

```javascript
// =============================================================
// SERVICE WORKER FOR OFFLINE SUPPORT
// File: public/sw.js

const CACHE_NAME = 'bizclaw-v1';
const STATIC_ASSETS = [
  '/',
  '/index.html',
  '/app.js',
  '/styles.css',
  '/vendor/preact.mjs',
];

// Install event
self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME)
      .then(cache => cache.addAll(STATIC_ASSETS))
      .then(() => self.skipWaiting())
  );
});

// Fetch event with stale-while-revalidate
self.addEventListener('fetch', (event) => {
  event.respondWith(
    caches.match(event.request)
      .then(cached => {
        const fetched = fetch(event.request)
          .then(response => {
            if (response.ok) {
              const clone = response.clone();
              caches.open(CACHE_NAME)
                .then(cache => cache.put(event.request, clone));
            }
            return response;
          })
          .catch(() => cached);
        
        return cached || fetched;
      })
  );
});

// =============================================================
// HTTP CACHE HEADERS (Server-side)
// File: server.rs or nginx.conf

// Nginx configuration
location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg|woff|woff2)$ {
    expires 30d;
    add_header Cache-Control "public, immutable";
}
```

### ✅ Week 2 Deliverables

| Deliverable | Target | Measurement |
|-------------|--------|-------------|
| Bundle Size Reduction | -30% | <300 KB total |
| First Contentful Paint | <1.8s | Lighthouse |
| Lazy Loading Implementation | 100% images | Manual test |
| Service Worker | ✅ Active | DevTools |

---

## WEEK 3: Responsive Design & PWA Features

### 📅 Ngày 1-2: Responsive Enhancement

```css
/* =============================================================
   RESPONSIVE DESIGN ENHANCEMENTS
   File: styles.css, dashboard-new.css
   ============================================================= */

/* Extra Small Devices (320px) */
@media (max-width: 374px) {
  /* Minimum supported width */
  :root {
    --base-font-size: 12px;
  }
  
  .stats-grid {
    grid-template-columns: 1fr;
    gap: 10px;
  }
  
  .chat-layout {
    display: none; /* Show simplified mobile chat */
  }
  
  .mobile-chat {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }
}

/* Small Devices (375px - 767px) */
@media (min-width: 375px) and (max-width: 767px) {
  /* Touch target minimum 44x44px */
  .btn, .nav-item, button {
    min-height: 44px;
    min-width: 44px;
  }
  
  /* Better form inputs for touch */
  .form-row input,
  .form-row select,
  .form-row textarea {
    min-height: 48px;
    font-size: 16px; /* Prevent iOS zoom */
  }
}

/* Tablet (768px - 1023px) */
@media (min-width: 768px) and (max-width: 1023px) {
  .app {
    grid-template-columns: 200px 1fr;
  }
  
  .stats-grid {
    grid-template-columns: repeat(3, 1fr);
  }
  
  .chat-layout {
    grid-template-columns: 280px 1fr;
  }
}

/* Desktop (1024px - 1439px) */
@media (min-width: 1024px) and (max-width: 1439px) {
  .app {
    grid-template-columns: 220px 1fr;
  }
  
  .stats-grid {
    grid-template-columns: repeat(4, 1fr);
  }
}

/* Large Desktop (1440px+) */
@media (min-width: 1440px) {
  .main-content {
    max-width: 1600px;
    margin: 0 auto;
  }
  
  .stats-grid {
    grid-template-columns: repeat(5, 1fr);
  }
}

/* Print Styles */
@media print {
  .sidebar, .mobile-nav, .chat-input {
    display: none !important;
  }
  
  body {
    background: white;
    color: black;
  }
}
```

### 📅 Ngày 3-4: PWA Implementation

```javascript
// =============================================================
// PWA MANIFEST
// File: public/manifest.json

{
  "name": "BizClaw Dashboard",
  "short_name": "BizClaw",
  "description": "OmniChannel AI Agent Platform",
  "start_url": "/",
  "display": "standalone",
  "background_color": "#08090d",
  "theme_color": "#6366f1",
  "orientation": "portrait-primary",
  "icons": [
    {
      "src": "/icons/icon-72.png",
      "sizes": "72x72",
      "type": "image/png"
    },
    {
      "src": "/icons/icon-192.png",
      "sizes": "192x192",
      "type": "image/png"
    },
    {
      "src": "/icons/icon-512.png",
      "sizes": "512x512",
      "type": "image/png"
    }
  ],
  "categories": ["business", "productivity"],
  "shortcuts": [
    {
      "name": "New Chat",
      "short_name": "Chat",
      "url": "/?page=chat",
      "icons": [{ "src": "/icons/chat.png" }]
    }
  ]
}

// =============================================================
// PWA SERVICE WORKER ENHANCEMENTS
// File: public/sw.js

// Handle offline fallback
self.addEventListener('fetch', (event) => {
  if (event.request.mode === 'navigate') {
    event.respondWith(
      fetch(event.request)
        .catch(() => {
          return caches.match('/offline.html');
        })
    );
  }
});

// Background sync for failed requests
self.addEventListener('sync', (event) => {
  if (event.tag === 'sync-messages') {
    event.waitUntil(syncMessages());
  }
});

async function syncMessages() {
  const pending = await getPendingMessages();
  for (const msg of pending) {
    try {
      await fetch('/api/messages', {
        method: 'POST',
        body: JSON.stringify(msg)
      });
      await removePendingMessage(msg.id);
    } catch (e) {
      console.error('Sync failed:', e);
    }
  }
}
```

### 📅 Ngày 5-7: Core Web Vitals Optimization

```javascript
// =============================================================
// LCP OPTIMIZATION
// File: shared.js

// Preload critical resources
function preloadCriticalResources() {
  // Preload LCP image
  const lcpImage = document.querySelector('.hero-image');
  if (lcpImage) {
    const link = document.createElement('link');
    link.rel = 'preload';
    link.as = 'image';
    link.href = lcpImage.src;
    document.head.appendChild(link);
  }
  
  // Preload critical CSS
  const criticalCSS = document.createElement('link');
  criticalCSS.rel = 'preload';
  criticalCSS.as = 'style';
  criticalCSS.href = '/styles.css';
  document.head.appendChild(criticalCSS);
}

// =============================================================
// CLS OPTIMIZATION
// File: shared.js

// Reserve space for dynamic content
const imageObserver = new MutationObserver((mutations) => {
  mutations.forEach(mutation => {
    mutation.addedNodes.forEach(node => {
      if (node.tagName === 'IMG') {
        // Set explicit dimensions
        if (!node.width && !node.style.width) {
          node.style.width = '100%';
          node.style.height = 'auto';
        }
      }
    });
  });
});

// =============================================================
// FID OPTIMIZATION
// File: shared.js

// Break up long tasks
async function processInChunks(items, processor, chunkSize = 50) {
  const results = [];
  for (let i = 0; i < items.length; i += chunkSize) {
    const chunk = items.slice(i, i + chunkSize);
    results.push(...chunk.map(processor));
    // Yield to main thread
    await new Promise(resolve => setTimeout(resolve, 0));
  }
  return results;
}
```

### ✅ Week 3 Deliverables

| Deliverable | Target | Measurement |
|-------------|--------|-------------|
| Responsive Validation | All breakpoints | BrowserStack |
| PWA Installation | ✅ Installable | Chrome DevTools |
| Offline Support | ✅ Working | Airplane mode test |
| LCP | <2.5s | Lighthouse |
| CLS | <0.1 | Lighthouse |

---

## WEEK 4: WCAG 2.1 AA Compliance & A/B Testing

### 📅 Ngày 1-2: Full WCAG 2.1 AA Audit

```
┌─────────────────────────────────────────────────────────────────┐
│ WCAG 2.1 AA CHECKLIST                                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│ 1.1 Non-text Content (Level A)                                  │
│ ───────────────────────────────────                             │
│ □ All images have alt text                                       │
│ □ Icon buttons have aria-label                                   │
│ □ Decorative images use alt=""                                  │
│ □ Complex images have long descriptions                          │
│                                                                 │
│ 1.3.1 Info and Relationships (Level A)                          │
│ ─────────────────────────────────────────                       │
│ □ Proper heading hierarchy (h1→h6)                               │
│ □ Tables have proper headers                                     │
│ □ Forms have associated labels                                    │
│ □ Lists use semantic list elements                               │
│                                                                 │
│ 1.4.1 Use of Color (Level A)                                    │
│ ────────────────────────────                                     │
│ □ Color is not sole means of conveying information               │
│ □ Visual indicators for all states                               │
│                                                                 │
│ 1.4.3 Contrast (Minimum) (Level AA)                            │
│ ────────────────────────────────────                            │
│ □ Normal text: 4.5:1 ratio minimum                              │
│ □ Large text (18pt+): 3:1 ratio minimum                         │
│ □ UI components and graphics: 3:1 minimum                        │
│                                                                 │
│ 2.1.1 Keyboard (Level A)                                         │
│ ──────────────────────────                                       │
│ □ All functionality via keyboard                                  │
│ □ No keyboard traps                                              │
│ □ Focus order is logical                                         │
│                                                                 │
│ 2.4.1 Bypass Blocks (Level A)                                   │
│ ───────────────────────────────                                   │
│ □ Skip to main content link                                      │
│ □ Skip to navigation link                                        │
│ □ Landmark regions                                                │
│                                                                 │
│ 2.4.3 Focus Order (Level A)                                     │
│ ─────────────────────────────                                   │
│ □ Focus follows logical sequence                                 │
│ □ DOM order matches visual order                                 │
│                                                                 │
│ 2.4.4 Link Purpose (Level A)                                    │
│ ──────────────────────────────                                   │
│ □ Link text is descriptive                                       │
│ □ Links make sense out of context                                │
│                                                                 │
│ 3.1.1 Language of Page (Level A)                                │
│ ───────────────────────────────────                             │
│ □ html lang attribute set                                        │
│                                                                 │
│ 3.2.2 On Input (Level A)                                       │
│ ───────────────────────────────                                  │
│ □ No context changes on input alone                              │
│                                                                 │
│ 3.3.1 Error Identification (Level A)                            │
│ ─────────────────────────────────────                           │
│ □ Errors clearly identified                                      │
│ □ Error messages are descriptive                                 │
│                                                                 │
│ 3.3.2 Labels or Instructions (Level A)                          │
│ ───────────────────────────────────────                         │
│ □ Labels provided for inputs                                      │
│ □ Instructions for complex inputs                                 │
│                                                                 │
│ 4.1.1 Parsing (Level A)                                        │
│ ──────────────────────────                                     │
│ □ Valid HTML                                                     │
│ □ Unique IDs                                                     │
│ □ Proper nesting                                                 │
│                                                                 │
│ 4.1.2 Name, Role, Value (Level A)                               │
│ ───────────────────────────────────                             │
│ □ All UI components have accessible names                         │
│ □ Roles are correct                                              │
│ □ Values are settable where applicable                           │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 📅 Ngày 3-4: Color Contrast Fixes

```css
/* =============================================================
   COLOR CONTRAST FIXES
   File: styles.css
   ============================================================= */

/* Fix secondary text contrast (was 5.2:1, need 4.5:1) */
/* Old: #7c8599 on #08090d = 5.2:1 (OK but borderline) */
/* Keep as is for dark theme */

/* Fix badge contrast issues */
/* Light theme badges needed darker backgrounds */

html.light .badge-green {
  background: rgba(5, 150, 105, 0.15);  /* Darker */
  color: #047857;  /* Darker green */
}

html.light .badge-red {
  background: rgba(220, 38, 38, 0.12);
  color: #b91c1c;
}

html.light .badge-warning {
  background: rgba(217, 119, 6, 0.12);
  color: #92400e;
}

/* Dark theme badges - already pass */
/* Verify with https://contrast-ratio.com */
```

### 📅 Ngày 5: A/B Testing Framework

```javascript
// =============================================================
// A/B TESTING FRAMEWORK
// File: analytics.js

class ABTesting {
  constructor() {
    this.experiments = {};
    this.userId = this.getUserId();
  }
  
  // Get or create persistent user ID
  getUserId() {
    let id = localStorage.getItem('bizclaw_user_id');
    if (!id) {
      id = 'user_' + Math.random().toString(36).substr(2, 9);
      localStorage.setItem('bizclaw_user_id', id);
    }
    return id;
  }
  
  // Deterministic bucketing
  bucket(userId, experimentId, variations) {
    const hash = this.hash(userId + experimentId);
    const bucket = hash % 100;
    
    let cumulative = 0;
    for (const [name, weight] of variations) {
      cumulative += weight;
      if (bucket < cumulative) {
        return name;
      }
    }
    return variations[0][0];
  }
  
  hash(str) {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      const char = str.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash;
    }
    return Math.abs(hash);
  }
  
  // Track experiment view
  trackView(experimentId, variation) {
    this.sendEvent('experiment_view', {
      experiment_id: experimentId,
      variation,
      user_id: this.userId,
      timestamp: Date.now()
    });
  }
  
  // Track conversion
  trackConversion(experimentId, goal, value = 1) {
    this.sendEvent('experiment_conversion', {
      experiment_id: experimentId,
      goal,
      value,
      user_id: this.userId,
      timestamp: Date.now()
    });
  }
  
  // Send to analytics
  sendEvent(name, data) {
    // Google Analytics 4
    if (window.gtag) {
      window.gtag('event', name, data);
    }
    
    // Custom analytics endpoint
    fetch('/api/analytics/events', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name, ...data })
    });
  }
  
  // Get variation for element
  getVariation(elementId, experimentId, variations) {
    const variation = this.bucket(this.userId, experimentId, variations);
    const element = document.getElementById(elementId);
    
    if (element) {
      element.setAttribute('data-experiment', experimentId);
      element.setAttribute('data-variation', variation);
    }
    
    return variation;
  }
}

// =============================================================
// A/B TEST EXAMPLES

// Test 1: New Navigation vs Old
const navExperiment = ab.getVariation(
  'main-nav',
  'nav_redesign_2024',
  [['control', 50], ['redesign', 50]]
);

if (navExperiment === 'redesign') {
  document.body.classList.add('new-nav');
}

// Test 2: Accessibility improvements
const a11yExperiment = ab.getVariation(
  'main-content',
  'a11y_enhanced_2024',
  [['control', 50], ['enhanced', 50]]
);

if (a11yExperiment === 'enhanced') {
  document.body.classList.add('enhanced-a11y');
}

// Track user satisfaction
document.querySelectorAll('.interactive-element').forEach(el => {
  el.addEventListener('focus', () => {
    ab.trackConversion('a11y_enhanced_2024', 'keyboard_focus');
  });
});
```

### 📅 Ngày 6-7: User Testing & Documentation

```javascript
// =============================================================
// ACCESSIBILITY STATEMENT TEMPLATE
// File: /public/accessibility-statement.html

const accessibilityStatement = `
# BizClaw Accessibility Statement

## Our Commitment

BizClaw is committed to ensuring digital accessibility for people with disabilities. 
We continually improve the user experience for everyone and apply relevant 
accessibility standards.

## Conformance Status

The Web Content Accessibility Guidelines (WCAG) defines requirements for designers 
and developers to improve accessibility for people with disabilities. BizClaw 
conforms to WCAG 2.1 Level AA.

## Technical Specifications

Accessibility of BizClaw is reliant on the following technologies:
- HTML5
- CSS3
- JavaScript (ES6+)
- ARIA 1.1

These technologies are relied upon for conformance with the accessibility standards used.

## Assessment Approach

BizClaw assessed the accessibility of this website using:
1. axe DevTools automated testing
2. Manual keyboard navigation testing
3. Screen reader testing (NVDA, VoiceOver, TalkBack)
4. Color contrast analyzer

## Known Issues

We are aware of the following issues:
1. Some third-party embeds may not meet accessibility standards
2. Complex data visualizations may require additional descriptions

## Feedback

We welcome your feedback on the accessibility of BizClaw. 
Please let us know if you encounter accessibility barriers:
- Email: accessibility@bizclaw.vn
- GitHub: https://github.com/nguyenduchoai/bizclaw/issues

## Date

This statement was last updated on April 16, 2026.
`;

// Save to file
export default accessibilityStatement;
```

### ✅ Week 4 Deliverables

| Deliverable | Status |
|-------------|--------|
| WCAG 2.1 AA Compliance | ⬜ |
| Accessibility Statement | ⬜ |
| A/B Testing Framework | ⬜ |
| User Testing Report | ⬜ |
| Training Materials | ⬜ |

---

## 📊 SUCCESS METRICS & KPIs

### Target Metrics

| Metric | Baseline | Week 2 | Week 4 | Target |
|--------|----------|---------|--------|--------|
| **Lighthouse Performance** | 85 | 90 | 95 | 95 |
| **Lighthouse Accessibility** | 72 | 85 | 95 | 95 |
| **First Contentful Paint** | 2.1s | 1.6s | 1.2s | <1.5s |
| **Largest Contentful Paint** | 3.2s | 2.6s | 2.2s | <2.5s |
| **Cumulative Layout Shift** | 0.15 | 0.12 | 0.08 | <0.1 |
| **Bundle Size (gzipped)** | 121 KB | 95 KB | 80 KB | <100 KB |
| **Keyboard Accessibility** | 60% | 85% | 100% | 100% |
| **Screen Reader Compatible** | 50% | 80% | 95% | 95% |
| **Color Contrast Pass** | 80% | 95% | 100% | 100% |

### Measurement Tools

```bash
# Lighthouse CLI
npm install -g lighthouse
lighthouse http://localhost:3000 --output=html --output-path=./lighthouse-report.html

# axe-core
npm install @axe-core/cli
axe http://localhost:3000 --exit

# WAVE
npx wave-browser chrome

# Accessibility Insights
npx accessibility-insights-test --url http://localhost:3000
```

---

## 📅 IMPLEMENTATION TIMELINE

```
Week 1: Critical Fixes & Foundation
──────────────────────────────────
Day 1-2: Bug analysis, prioritization
Day 3-4: Accessibility core features
Day 5: Accessibility audit
Day 6-7: Bug fixes

Week 2: Performance Optimization  
──────────────────────────────────
Day 1-2: Performance audit, analysis
Day 3-4: Image optimization, lazy loading
Day 5: Code splitting, bundling
Day 6-7: Caching, service worker

Week 3: Responsive & PWA
──────────────────────────────────
Day 1-2: Responsive enhancements
Day 3-4: PWA implementation
Day 5-7: Core Web Vitals optimization

Week 4: WCAG Compliance & Testing
──────────────────────────────────
Day 1-2: Full WCAG 2.1 AA audit
Day 3-4: Color contrast fixes, A/B framework
Day 5-7: User testing, documentation
```

---

## 📁 DELIVERABLES CHECKLIST

### Week 1
- [ ] Bug fix documentation (`/docs/week1-bugfix-log.md`)
- [ ] Resolved issue tracker (`/docs/week1-issues-resolved.md`)
- [ ] Basic accessibility checklist (`/docs/week1-a11y-checklist.md`)
- [ ] axe-core audit report (`/docs/week1-axe-report.md`)

### Week 2
- [ ] Performance audit report (`/docs/week2-performance-audit.md`)
- [ ] Before/after metrics comparison (`/docs/week2-metrics.md`)
- [ ] Bundle optimization documentation (`/docs/week2-bundle-optimization.md`)

### Week 3
- [ ] Responsive validation report (`/docs/week3-responsive-validation.md`)
- [ ] PWA implementation guide (`/docs/week3-pwa-guide.md`)
- [ ] Core Web Vitals report (`/docs/week3-webvitals.md`)

### Week 4
- [ ] WCAG 2.1 AA compliance certificate (`/docs/week4-wcag-certificate.md`)
- [ ] A/B testing results (`/docs/week4-ab-results.md`)
- [ ] Accessibility statement (`/public/accessibility-statement.html`)
- [ ] User testing report (`/docs/week4-user-testing.md`)
- [ ] Training materials (`/docs/week4-training-materials.md`)

---

## 🔧 RESOURCES & TOOLS

### Automated Testing
- **axe DevTools**: https://www.deque.com/axe/devtools/
- **Lighthouse**: https://developer.chrome.com/docs/lighthouse/
- **WAVE**: https://wave.webaim.org/
- **Accessibility Insights**: https://accessibilityinsights.io/

### Manual Testing
- **NVDA** (Windows): https://www.nvaccess.org/
- **VoiceOver** (macOS/iOS): Built-in
- **TalkBack** (Android): Built-in
- **Keyboard Navigation**: Tab, Shift+Tab, Arrow keys, Enter, Space, Escape

### Color Contrast
- **Contrast Ratio Calculator**: https://contrast-ratio.com/
- **Accessible Colors**: https://accessible-colors.com/

### Performance
- **WebPageTest**: https://www.webpagetest.org/
- **GTmetrix**: https://gtmetrix.com/
- **PageSpeed Insights**: https://pagespeed.web.dev/

---

*Roadmap created: 2026-04-16*  
*Next review: 2026-04-23*  
*Target completion: 2026-05-14*
