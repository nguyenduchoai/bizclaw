// BizClaw Dashboard — Shared utilities for page modules
// Exports: authFetch, authHeaders, t (i18n), StatsCard, Toast, ToastManager
// Version: 1.1.7+ (Accessibility & Performance improvements)

const { h, html, useState, useEffect, useContext, useCallback, useRef, useMemo } = window;

import { vi } from '/static/dashboard/i18n/vi.js';
import { en } from '/static/dashboard/i18n/en.js';

const I18N = { vi, en };

// ═══ I18N ═══
export function t(key, lang) {
  return (I18N[lang] || I18N.vi)[key] || I18N.vi[key] || key;
}

// ═══ AUTH HELPERS ═══
function getJwtToken() {
  const url = new URL(location.href);
  const tokenParam = url.searchParams.get('token');
  if (tokenParam) {
    sessionStorage.setItem('bizclaw_jwt', tokenParam);
    url.searchParams.delete('token');
    history.replaceState(null, '', url.pathname + url.search + url.hash);
    return tokenParam;
  }
  const match = document.cookie.match(/bizclaw_token=([^;]+)/);
  if (match) return match[1];
  return sessionStorage.getItem('bizclaw_jwt') || '';
}

let jwtToken = getJwtToken();

export function authHeaders(extra = {}) {
  return { ...extra, 'Authorization': 'Bearer ' + jwtToken, 'Content-Type': 'application/json' };
}

export async function authFetch(url, opts = {}) {
  if (!opts.headers) opts.headers = {};
  if (jwtToken) {
    opts.headers['Authorization'] = 'Bearer ' + jwtToken;
  }
  const res = await fetch(url, opts);
  if (res.status === 401) {
    sessionStorage.removeItem('bizclaw_jwt');
    jwtToken = '';
    throw new Error('Unauthorized');
  }
  return res;
}

export function refreshJwtToken() {
  jwtToken = getJwtToken();
}

export function getToken() {
  return jwtToken;
}

export function setToken(newToken) {
  jwtToken = newToken;
}

// ═══ ACCESSIBILITY UTILITIES ═══

// Skip to main content link (must be called on page load)
export function initAccessibility() {
  // Add skip link if not exists
  if (!document.querySelector('.skip-link')) {
    const skipLink = document.createElement('a');
    skipLink.href = '#main-content';
    skipLink.className = 'skip-link';
    skipLink.textContent = 'Skip to main content';
    skipLink.addEventListener('click', (e) => {
      e.preventDefault();
      const main = document.getElementById('main-content') || document.querySelector('main');
      if (main) {
        main.setAttribute('tabindex', '-1');
        main.focus();
      }
    });
    document.body.insertBefore(skipLink, document.body.firstChild);
  }

  // Add focus indicators CSS if not exists
  if (!document.getElementById('a11y-focus-styles')) {
    const style = document.createElement('style');
    style.id = 'a11y-focus-styles';
    style.textContent = `
      .skip-link {
        position: absolute;
        top: -40px;
        left: 0;
        background: var(--accent, #6366f1);
        color: white;
        padding: 12px 24px;
        z-index: 10000;
        transition: top 0.3s;
        font-weight: 600;
        text-decoration: none;
        border-radius: 0 0 8px 0;
      }
      .skip-link:focus {
        top: 0;
        outline: 2px solid var(--accent2, #818cf8);
        outline-offset: 2px;
      }
      *:focus-visible {
        outline: 2px solid var(--accent, #6366f1);
        outline-offset: 2px;
      }
      button:focus-visible,
      a:focus-visible,
      input:focus-visible,
      select:focus-visible,
      textarea:focus-visible,
      [tabindex]:focus-visible {
        outline: 2px solid var(--accent, #6366f1);
        outline-offset: 2px;
      }
    `;
    document.head.appendChild(style);
  }

  // Mark main content with role="main"
  const main = document.querySelector('main') || document.getElementById('main-content');
  if (main && !main.hasAttribute('role')) {
    main.setAttribute('role', 'main');
    main.id = main.id || 'main-content';
  }

  // Optimize images for lazy loading
  optimizeImages();

  // Initialize toast manager
  window.toastManager = new ToastManager();
}

// ═══ IMAGE LAZY LOADING ═══
export function optimizeImages() {
  // Add loading="lazy" to images without it
  document.querySelectorAll('img:not([loading])').forEach(img => {
    img.setAttribute('loading', 'lazy');
    img.setAttribute('decoding', 'async');
  });

  // Set explicit dimensions for CLS prevention
  document.querySelectorAll('img:not([width]):not([style*="width"])').forEach(img => {
    if (!img.width && img.naturalWidth) {
      img.width = img.naturalWidth;
    }
    if (!img.height && img.naturalHeight) {
      img.height = img.naturalHeight;
    }
  });
}

// ═══ TOAST MANAGER (FIX BUG-02) ═══
class ToastManager {
  constructor() {
    this.queue = [];
    this.maxVisible = 3;
    this.container = null;
    this.init();
  }

  init() {
    // Create toast container with ARIA
    this.container = document.getElementById('toast-container');
    if (!this.container) {
      this.container = document.createElement('div');
      this.container.id = 'toast-container';
      this.container.setAttribute('role', 'status');
      this.container.setAttribute('aria-live', 'polite');
      this.container.setAttribute('aria-atomic', 'true');
      this.container.className = 'toast-container';
      document.body.appendChild(this.container);
    }
  }

  show(message, type = 'info', duration = 5000) {
    const id = 'toast-' + Date.now() + '-' + Math.random().toString(36).substr(2, 9);
    const toast = { id, message, type, duration };
    this.queue.push(toast);
    this.render();

    if (duration > 0) {
      setTimeout(() => this.dismiss(id), duration);
    }
    return id;
  }

  success(message, duration = 5000) {
    return this.show(message, 'success', duration);
  }

  error(message, duration = 8000) {
    return this.show(message, 'error', duration);
  }

  info(message, duration = 5000) {
    return this.show(message, 'info', duration);
  }

  warning(message, duration = 6000) {
    return this.show(message, 'warning', duration);
  }

  dismiss(id) {
    const element = document.getElementById(id);
    if (element) {
      element.classList.add('toast-hide');
      setTimeout(() => {
        this.queue = this.queue.filter(t => t.id !== id);
        this.render();
      }, 300);
    } else {
      this.queue = this.queue.filter(t => t.id !== id);
      this.render();
    }
  }

  dismissAll() {
    this.queue = [];
    this.render();
  }

  render() {
    const visible = this.queue.slice(0, this.maxVisible);
    this.container.innerHTML = visible.map(toast => {
      const icons = {
        success: '✓',
        error: '✕',
        warning: '⚠',
        info: 'ℹ'
      };
      const colors = {
        success: 'var(--green, #34d399)',
        error: 'var(--red, #ef4444)',
        warning: 'var(--orange, #fb923c)',
        info: 'var(--accent2, #818cf8)'
      };
      return `
        <div id="${toast.id}" 
             class="toast" 
             role="alert"
             aria-live="assertive"
             style="border-left: 3px solid ${colors[toast.type] || colors.info}">
          <span style="margin-right:8px;color:${colors[toast.type] || colors.info}">${icons[toast.type] || icons.info}</span>
          <span>${toast.message}</span>
          <button onclick="window.toastManager && window.toastManager.dismiss('${toast.id}')" 
                  class="toast-close"
                  aria-label="Close notification"
                  style="background:none;border:none;cursor:pointer;margin-left:12px;opacity:0.6;font-size:16px">✕</button>
        </div>
      `;
    }).join('');

    // Announce to screen readers for first toast
    if (visible.length > 0 && !this.lastAnnounced) {
      this.lastAnnounced = visible[0].message;
      const announcement = document.createElement('div');
      announcement.setAttribute('aria-live', 'polite');
      announcement.setAttribute('aria-atomic', 'true');
      announcement.className = 'sr-only';
      announcement.textContent = visible[0].message;
      document.body.appendChild(announcement);
      setTimeout(() => announcement.remove(), 1000);
    }
  }
}

// Add toast CSS if not exists
if (!document.getElementById('toast-styles')) {
  const toastStyle = document.createElement('style');
  toastStyle.id = 'toast-styles';
  toastStyle.textContent = `
    .toast-container {
      position: fixed;
      bottom: 24px;
      right: 24px;
      z-index: 9999;
      display: flex;
      flex-direction: column;
      gap: 8px;
      max-width: 400px;
    }
    .toast {
      display: flex;
      align-items: center;
      padding: 14px 18px;
      background: var(--surface, #12151e);
      border: 1px solid var(--border, #1e2433);
      border-radius: 10px;
      font-size: 13px;
      box-shadow: 0 8px 32px rgba(0,0,0,0.4);
      animation: toastSlideIn 0.3s cubic-bezier(0.16, 1, 0.3, 1);
      backdrop-filter: blur(12px);
    }
    .toast-hide {
      animation: toastSlideOut 0.3s ease-out forwards;
    }
    .toast-close:hover {
      opacity: 1 !important;
    }
    @keyframes toastSlideIn {
      from { opacity: 0; transform: translateY(16px) scale(0.96); }
      to { opacity: 1; transform: translateY(0) scale(1); }
    }
    @keyframes toastSlideOut {
      from { opacity: 1; transform: translateX(0); }
      to { opacity: 0; transform: translateX(100%); }
    }
    .sr-only {
      position: absolute;
      width: 1px;
      height: 1px;
      padding: 0;
      margin: -1px;
      overflow: hidden;
      clip: rect(0, 0, 0, 0);
      white-space: nowrap;
      border: 0;
    }
  `;
  document.head.appendChild(toastStyle);
}

// Legacy Toast function (backward compatibility)
export function Toast({ message, type }) {
  if (!message) return null;
  const colors = { error: 'var(--red)', success: 'var(--green)', info: 'var(--accent2)' };
  return html`<div class="toast" role="alert" aria-live="assertive" style="border-left: 3px solid ${colors[type] || colors.info}">
    ${message}
  </div>`;
}

// ═══ STATS CARD ═══
export function StatsCard({ label, value, color = 'accent', sub, icon }) {
  return html`<div class="card stats-card" role="region" aria-label="${label}">
    <div style="display:flex;align-items:center;gap:8px;margin-bottom:8px">
      ${icon ? html`<span aria-hidden="true" style="font-size:1.3em">${icon}</span>` : null}
      <span class="stats-label">${label}</span>
    </div>
    <div class="stats-value" style="color:var(--${color})" aria-label="${value}">${value}</div>
    ${sub ? html`<div style="font-size:0.75em;opacity:0.6;margin-top:4px">${sub}</div>` : null}
  </div>`;
}

// ═══ MOBILE SIDEBAR MANAGER (FIX BUG-01) ═══
export function toggleMobileSidebar(open) {
  const sidebar = document.querySelector('.sidebar');
  const backdrop = document.querySelector('.sidebar-backdrop');

  if (open) {
    sidebar.classList.add('open');
    if (backdrop) {
      backdrop.style.display = 'block';
      backdrop.classList.add('show');
      backdrop.setAttribute('aria-hidden', 'false');
      backdrop.onclick = () => toggleMobileSidebar(false);
    }
    // Trap focus within sidebar
    trapFocus(sidebar);
    // Prevent body scroll
    document.body.style.overflow = 'hidden';
    // Add keyboard listener
    document.addEventListener('keydown', handleSidebarKeydown);
  } else {
    sidebar.classList.remove('open');
    if (backdrop) {
      backdrop.classList.remove('show');
      backdrop.setAttribute('aria-hidden', 'true');
      setTimeout(() => {
        backdrop.style.display = 'none';
      }, 300);
    }
    // Restore body scroll
    document.body.style.overflow = '';
    // Remove keyboard listener
    document.removeEventListener('keydown', handleSidebarKeydown);
  }
}

function handleSidebarKeydown(e) {
  if (e.key === 'Escape') {
    toggleMobileSidebar(false);
  }
}

function trapFocus(element) {
  const focusable = element.querySelectorAll(
    'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])'
  );
  if (focusable.length === 0) return;

  const first = focusable[0];
  const last = focusable[focusable.length - 1];

  function handleTab(e) {
    if (e.key !== 'Tab') return;

    if (e.shiftKey) {
      if (document.activeElement === first) {
        e.preventDefault();
        last.focus();
      }
    } else {
      if (document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    }
  }

  element.addEventListener('keydown', handleTab);
  first.focus();
}

// ═══ BUTTON DEBOUNCE (FIX BUG-08) ═══
export function debounceButton(button, callback, delay = 500) {
  if (button._debounceTimeout) {
    return false;
  }
  button._debounceTimeout = setTimeout(() => {
    button._debounceTimeout = null;
  }, delay);
  callback();
  return true;
}

// Auto-apply to buttons with data-debounce attribute
export function initButtonDebounce() {
  document.addEventListener('click', (e) => {
    const button = e.target.closest('button[data-debounce]');
    if (button) {
      const delay = parseInt(button.dataset.debounce) || 500;
      if (!debounceButton(button, () => {}, delay)) {
        e.preventDefault();
        e.stopPropagation();
      }
    }
  });
}

// ═══ LAZY LOADING OBSERVER ═══
export function initLazyLoading() {
  if (!('IntersectionObserver' in window)) {
    // Fallback for older browsers
    document.querySelectorAll('[data-src]').forEach(img => {
      if (img.dataset.src) {
        img.src = img.dataset.src;
      }
    });
    return;
  }

  const observer = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        const el = entry.target;
        if (el.dataset.src) {
          el.src = el.dataset.src;
          el.removeAttribute('data-src');
        }
        if (el.dataset.srcset) {
          el.srcset = el.dataset.srcset;
          el.removeAttribute('data-srcset');
        }
        observer.unobserve(el);
      }
    });
  }, {
    rootMargin: '50px 0px',
    threshold: 0.01
  });

  document.querySelectorAll('[data-src], [data-srcset]').forEach(el => {
    observer.observe(el);
  });
}

// ═══ A/B TESTING FRAMEWORK ═══
class ABTesting {
  constructor() {
    this.experiments = {};
    this.userId = this.getUserId();
    this.events = [];
  }

  getUserId() {
    let id = localStorage.getItem('bizclaw_user_id');
    if (!id) {
      id = 'user_' + Math.random().toString(36).substr(2, 9) + '_' + Date.now();
      localStorage.setItem('bizclaw_user_id', id);
    }
    return id;
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

  getVariation(experimentId, variations) {
    const variation = this.bucket(this.userId, experimentId, variations);
    this.trackView(experimentId, variation);
    return variation;
  }

  trackView(experimentId, variation) {
    this.events.push({
      type: 'view',
      experiment_id: experimentId,
      variation,
      user_id: this.userId,
      timestamp: Date.now()
    });
    console.debug(`[A/B] View: ${experimentId} = ${variation}`);
  }

  trackConversion(experimentId, goal, value = 1) {
    this.events.push({
      type: 'conversion',
      experiment_id: experimentId,
      goal,
      value,
      user_id: this.userId,
      timestamp: Date.now()
    });
    console.debug(`[A/B] Conversion: ${experimentId} - ${goal}`);
    this.sendEvents();
  }

  sendEvents() {
    if (this.events.length === 0) return;
    const eventsToSend = this.events.splice(0, 10);
    fetch('/api/analytics/ab-events', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ events: eventsToSend })
    }).catch(() => {});
  }
}

export const abTesting = new ABTesting();

// ═══ PERFORMANCE MONITORING ═══
export function initPerformanceMonitoring() {
  if ('PerformanceObserver' in window) {
    // LCP
    new PerformanceObserver((list) => {
      const entries = list.getEntries();
      const lastEntry = entries[entries.length - 1];
      console.debug(`[Performance] LCP: ${lastEntry.startTime.toFixed(2)}ms`);
    }).observe({ entryTypes: ['largest-contentful-paint'] });

    // FID
    new PerformanceObserver((list) => {
      list.getEntries().forEach(entry => {
        console.debug(`[Performance] FID: ${entry.processingStart - entry.startTime.toFixed(2)}ms`);
      });
    }).observe({ entryTypes: ['first-input'] });

    // CLS
    let clsValue = 0;
    new PerformanceObserver((list) => {
      list.getEntries().forEach(entry => {
        if (!entry.hadRecentInput) {
          clsValue += entry.value;
          console.debug(`[Performance] CLS: ${clsValue.toFixed(4)}`);
        }
      });
    }).observe({ entryTypes: ['layout-shift'] });
  }
}

// ═══ PAGE DEFINITIONS ═══
export const PAGES = [
  { id: 'dashboard', icon: '📊', label: 'nav.dashboard' },
  { id: 'sep_ops', sep: true, groupLabel: 'nav.group_ops' },
  { id: 'chat', icon: '💬', label: 'nav.webchat' },
  { id: 'handoff', icon: '🤝', label: 'nav.handoff' },
  { id: 'campaigns', icon: '📢', label: 'nav.campaigns' },
  { id: 'hands', icon: '🤚', label: 'nav.hands' },
  { id: 'workflows', icon: '🔄', label: 'nav.workflows' },
  { id: 'scheduler', icon: '⏰', label: 'nav.scheduler' },
  { id: 'sep_biz', sep: true, groupLabel: 'nav.group_biz' },
  { id: 'products', icon: '🛍️', label: 'nav.products' },
  { id: 'paymentlinks', icon: '💳', label: 'nav.paymentlinks' },
  { id: 'analytics', icon: '📈', label: 'nav.analytics' },
  { id: 'sep_intel', sep: true, groupLabel: 'nav.group_intel' },
  { id: 'agents', icon: '🤖', label: 'nav.agents' },
  { id: 'knowledge', icon: '📚', label: 'nav.knowledge' },
  { id: 'wiki', icon: '📖', label: 'nav.wiki' },
  { id: 'gallery', icon: '📦', label: 'nav.gallery' },
  { id: 'plugins', icon: '🛒', label: 'nav.plugins' },
  { id: 'sep_cfg', sep: true, groupLabel: 'nav.group_cfg' },
  { id: 'channels', icon: '📱', label: 'nav.channels' },
  { id: 'providers', icon: '🔌', label: 'nav.providers' },
  { id: 'tools', icon: '🛠️', label: 'nav.tools' },
  { id: 'mcp', icon: '🔗', label: 'nav.mcp' },
  { id: 'settings', icon: '⚙️', label: 'nav.settings' },
];

// ═══ NEW HORIZONTAL MENU ═══
export const PAGES_NEW = [
  {
    id: 'home', label: 'Trang chủ', icon: '🏠',
    children: [
      { id: 'dashboard', icon: '📊', label: 'Bảng điều khiển' },
      { id: 'analytics', icon: '📈', label: 'Phân tích' },
      { id: 'activity', icon: '⚡', label: 'Hoạt động' },
    ]
  },
  {
    id: 'ai-team', label: 'Nhân sự AI', icon: '🤖',
    children: [
      { id: 'agents', icon: '🤖', label: 'Đội nhóm' },
      { id: 'teams', icon: '👥', label: 'Nhóm Agent' },
      { id: 'skills', icon: '💼', label: 'Kỹ năng' },
      { id: 'brain', icon: '🧠', label: 'Bộ nhớ AI' },
      { id: 'gallery', icon: '📦', label: 'Mẫu Agent' },
    ]
  },
  {
    id: 'channels', label: 'Kênh liên lạc', icon: '📱',
    children: [
      { id: 'channels', icon: '📱', label: 'Tất cả kênh' },
      { id: 'webhooks', icon: '🪝', label: 'Webhook' },
      { id: 'mcp', icon: '🔗', label: 'Máy chủ MCP' },
    ]
  },
  {
    id: 'automation', label: 'Tự động hóa', icon: '🔄',
    children: [
      { id: 'workflows', icon: '🔄', label: 'Quy trình' },
      { id: 'scheduler', icon: '⏰', label: 'Lịch trình' },
      { id: 'hands', icon: '🤚', label: 'Tay Robot' },
    ]
  },
  {
    id: 'operations', label: 'Vận hành', icon: '💬',
    children: [
      { id: 'chat', icon: '💬', label: 'Trò chuyện' },
      { id: 'handoff', icon: '🤝', label: 'Chuyển giao' },
      { id: 'campaigns', icon: '📢', label: 'Chiến dịch' },
      { id: 'knowledge', icon: '📚', label: 'Tri thức' },
      { id: 'products', icon: '📦', label: 'Sản phẩm' },
      { id: 'tiktok', icon: '🎵', label: 'TikTok' },
      { id: 'shopee', icon: '🛒', label: 'Shopee' },
    ]
  },
  {
    id: 'system', label: 'Cấu hình', icon: '🔧',
    children: [
      { id: 'settings', icon: '⚙️', label: 'Cài đặt' },
      { id: 'providers', icon: '🔌', label: 'Nhà cung cấp' },
      { id: 'tools', icon: '🛠️', label: 'Công cụ' },
      { id: 'apikeys', icon: '🔑', label: 'API Keys' },
      { id: 'plugins', icon: '🧩', label: 'Plugin' },
      { id: 'configfile', icon: '📝', label: 'Tệp cấu hình' },
    ]
  },
  {
    id: 'help', label: 'Trợ giúp', icon: '❓',
    children: [
      { id: 'wiki', icon: '📖', label: 'Tài liệu' },
      { id: 'usage', icon: '📊', label: 'Sử dụng' },
      { id: 'cost', icon: '💰', label: 'Chi phí' },
    ]
  },
];

// ═══ SME MODE PAGES ═══
export const SME_PAGES = [
  { id: 'sme', icon: '🏠', label: 'nav.dashboard' },
  { id: 'agents', icon: '🤖', label: 'nav.agents' },
  { id: 'workflows', icon: '🔄', label: 'nav.workflows' },
  { id: 'analytics', icon: '📊', label: 'nav.analytics' },
  { id: 'settings', icon: '⚙️', label: 'nav.settings' },
];

// Make shared functions available globally
window.authFetch = authFetch;
window.authHeaders = authHeaders;
window.t = t;
window.SME_PAGES = SME_PAGES;
window.toggleMobileSidebar = toggleMobileSidebar;
window.abTesting = abTesting;

// ═══ AUTO-INIT ON LOAD ═══
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', () => {
    initAccessibility();
    initButtonDebounce();
    initLazyLoading();
    initPerformanceMonitoring();
  });
} else {
  initAccessibility();
  initButtonDebounce();
  initLazyLoading();
  initPerformanceMonitoring();
}
