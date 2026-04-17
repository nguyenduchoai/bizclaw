// BizClaw Dashboard — Main App Orchestrator
// Preact + HTM, no build step required
//
// Modular architecture:
//   app.js       — Core shell (routing, auth, WebSocket, sidebar)
//   shared.js    — Shared utilities (authFetch, i18n, components)
//   pages/*.js   — Individual page modules (lazy-loaded)
//
// CRITICAL: Do NOT import preact/hooks/htm here!
// Uses window.* globals set by index.html from a single import chain.

const { h, html, render, createContext,
        useState, useEffect, useContext, useCallback, useRef, useMemo } = window;

import { t, authFetch, authHeaders, Toast, StatsCard, PAGES, SME_PAGES, getToken, setToken, refreshJwtToken } from '/static/dashboard/shared.js';
import { OnboardingWizard } from '/static/dashboard/pages/onboarding.js';

// ═══ APP CONTEXT ═══
const AppContext = createContext({});

export function useApp() { return useContext(AppContext); }
// Make AppContext globally accessible for page modules
window.AppContext = AppContext;

// Get JWT token (use shared module)
let jwtToken = getToken();

// ═══ LAZY PAGE LOADER ═══
// Dynamic import with caching — each page loads only when navigated to
const pageCache = {};

async function loadPage(pageId) {
  if (pageCache[pageId]) return pageCache[pageId];

  const PAGE_MAP = {
    dashboard:     { file: 'dashboard.js',     export: 'DashboardPage' },
    chat:          { file: 'chat.js',          export: 'ChatPage' },
    hands:         { file: 'hands.js',         export: 'HandsPage' },
    settings:      { file: 'settings.js',      export: 'SettingsPage' },
    providers:     { file: 'providers.js',     export: 'ProvidersPage' },
    channels:      { file: 'channels.js',      export: 'ChannelsPage' },
    tools:         { file: 'tools.js',         export: 'ToolsPage' },
    mcp:           { file: 'mcp.js',           export: 'McpPage' },
    agents:        { file: 'agents.js?v=2',        export: 'AgentsPage' },
    knowledge:     { file: 'knowledge.js',     export: 'KnowledgePage' },
    kanban:        { file: 'kanban.js',        export: 'KanbanPage' },
    brain:         { file: 'settings.js',      export: 'SettingsPage' }, // brain → settings
    configfile:    { file: 'config_file.js',   export: 'ConfigFilePage' },
    scheduler:     { file: 'scheduler.js',     export: 'SchedulerPage' },
    cost:          { file: 'cost.js',          export: 'CostPage' },
    activity:      { file: 'activity.js',      export: 'ActivityPage' },
    workflows:     { file: 'workflows.js',     export: 'WorkflowsPage' },
    wiki:          { file: 'wiki.js',          export: 'WikiPage' },
    apikeys:       { file: 'api_keys.js',      export: 'ApiKeysPage' },
    usage:         { file: 'usage.js',         export: 'UsagePage' },
    analytics:     { file: 'analytics.js',     export: 'AnalyticsPage' },
    plugins:       { file: 'plugins.js',       export: 'PluginsPage' },
    gallery:       { file: 'gallery.js',       export: 'GalleryPage' },
    dbassistant:   { file: 'db_assistant.js',  export: 'DbAssistantPage' },
    campaigns:     { file: 'campaigns.js?v=2',     export: 'CampaignsPage' },
    products:      { file: 'products.js',      export: 'ProductsPage' },
    handoff:       { file: 'handoff.js?v=2',        export: 'HandoffPage' },
    paymentlinks:  { file: 'payment_links.js',  export: 'PaymentLinksPage' },
    cloud:         { file: 'cloud.js',          export: 'CloudPage' },
    sme:           { file: 'sme-dashboard.js',  export: 'SmeDashboardPage' },
  };

  const mapping = PAGE_MAP[pageId];
  if (!mapping) return null;

  try {
    const mod = await import(`/static/dashboard/pages/${mapping.file}`);
    const component = mod[mapping.export];
    pageCache[pageId] = component;
    return component;
  } catch (e) {
    console.error(`Failed to load page module: ${pageId}`, e);
    return null;
  }
}

// ═══ LAZY PAGE ROUTER ═══
function PageRouter({ page, config, lang }) {
  const [Component, setComponent] = useState(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setLoading(true);
    loadPage(page).then(comp => {
      setComponent(() => comp);
      setLoading(false);
    });
  }, [page]);

  if (loading) {
    return html`<div style="display:flex;align-items:center;justify-content:center;padding:60px;color:var(--text2)">
      <div style="text-align:center">
        <div style="font-size:32px;margin-bottom:12px;animation:pulse 1s infinite">⏳</div>
        <div>Loading...</div>
      </div>
    </div>`;
  }
  if (!Component) {
    return html`<div class="card" style="padding:40px;text-align:center">
      <div style="font-size:48px;margin-bottom:16px">📄</div>
      <h2>${page}</h2>
    </div>`;
  }
  return html`<${Component} config=${config} lang=${lang} />`;
}

// ═══ SIDEBAR ═══
function Sidebar({ currentPage, lang, wsStatus, agentName, theme, isOpen, onClose, smeMode, onToggleSme }) {
  return html`
    <div class="sidebar-backdrop ${isOpen ? 'show' : ''}" onClick=${onClose}></div>
    <aside class="sidebar ${isOpen ? 'open' : ''}">
    <div class="logo">
      <span class="icon">⚡</span>
      <span class="text">BizClaw</span>
    </div>
    <nav class="nav" id="sidebar-nav">
      ${PAGES.map(p => {
        if (p.sep) {
          return p.groupLabel
            ? html`<div class="nav-group-label" key=${p.id}>${t(p.groupLabel, lang)}</div>`
            : html`<div class="nav-sep" key=${p.id}></div>`;
        }
        return html`<a key=${p.id} href="/${p.id === 'dashboard' ? '' : p.id}"
              data-page=${p.id}
              class=${currentPage === p.id ? 'active' : ''}>
            ${p.icon} <span>${t(p.label, lang)}</span>
          </a>`;
      })}
    </nav>
    <div class="sidebar-footer">
      <div style="display:flex;align-items:center;gap:6px;margin-bottom:6px">
        <button data-lang="vi"
          style="padding:2px 8px;font-size:11px;border-radius:4px;border:1px solid var(--border);background:${lang === 'vi' ? 'var(--accent)' : 'transparent'};color:${lang === 'vi' ? '#fff' : 'var(--text2)'};cursor:pointer">VI</button>
        <button data-lang="en"
          style="padding:2px 8px;font-size:11px;border-radius:4px;border:1px solid var(--border);background:${lang === 'en' ? 'var(--accent)' : 'transparent'};color:${lang === 'en' ? '#fff' : 'var(--text2)'};cursor:pointer">EN</button>
      </div>
      <button class="theme-toggle" data-theme-toggle="true">
        ${theme === 'light' ? '🌙' : '☀️'} ${theme === 'light' ? 'Dark Mode' : 'Light Mode'}
      </button>
      <div style="margin-top:8px;padding-top:8px;border-top:1px solid var(--border)">
        <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:4px">
          <span style="font-size:11px;color:var(--text2)">SME Mode</span>
          <label class="switch" style="width:36px;height:20px">
            <input type="checkbox" checked=${smeMode} onChange=${e => onToggleSme(e.target.checked)} />
            <span class="slider" style="background:${smeMode ? 'var(--accent)' : 'var(--border)'}"></span>
          </label>
        </div>
      </div>
      <div id="ws-status-indicator">${wsStatus === 'connected' ? '🟢' : '🔴'} ${t(wsStatus === 'connected' ? 'status.connected' : 'status.disconnected', lang)}</div>
      <div style="margin-top:4px">${agentName}</div>
    </div>
  </aside>`;
}

// ═══ SME TOP BAR ═══
function SmeTopBar({ currentPage, lang, agentName, onNavigate, smeMode, onToggleSme }) {
  return html`
    <div class="sme-topbar">
      <div class="sme-logo">
        <span class="icon">👑</span>
        <span class="text">BizClaw SME</span>
      </div>
      <nav class="sme-nav">
        ${SME_PAGES.map(p => html`
          <button
            key=${p.id}
            data-page=${p.id}
            class="sme-nav-item ${currentPage === p.id ? 'active' : ''}"
            onClick=${() => onNavigate(p.id)}
          >
            <span class="emoji">${p.icon}</span>
            <span>${t(p.label, lang)}</span>
          </button>
        `)}
      </nav>
      <div style="display:flex;align-items:center;gap:12px;margin-left:auto">
        <div style="display:flex;align-items:center;gap:6px">
          <button data-lang="vi"
            style="padding:2px 8px;font-size:11px;border-radius:4px;border:1px solid var(--border);background:${lang === 'vi' ? 'var(--accent)' : 'transparent'};color:${lang === 'vi' ? '#fff' : 'var(--text2)'};cursor:pointer">VI</button>
          <button data-lang="en"
            style="padding:2px 8px;font-size:11px;border-radius:4px;border:1px solid var(--border);background:${lang === 'en' ? 'var(--accent)' : 'transparent'};color:${lang === 'en' ? '#fff' : 'var(--text2)'};cursor:pointer">EN</button>
        </div>
        <label class="switch" style="display:flex;align-items:center">
          <input type="checkbox" checked=${smeMode} onChange=${e => onToggleSme(e.target.checked)} />
          <span class="slider" style="background:${smeMode ? 'var(--accent)' : 'var(--border)'}"></span>
        </label>
      </div>
    </div>
  `;
}

// ═══ AUTH GATE ═══
function AuthGate({ onSuccess }) {
  return html`<div style="position:fixed;inset:0;background:var(--bg);z-index:300;display:flex;align-items:center;justify-content:center">
    <div style="background:var(--surface);border:1px solid var(--border);border-radius:16px;padding:40px;width:380px;text-align:center">
      <div style="font-size:32px;margin-bottom:12px">🔐</div>
      <h2 style="color:var(--accent);margin-bottom:8px">BizClaw Agent</h2>
      <p style="color:var(--text2);font-size:13px;margin-bottom:24px">Phiên đăng nhập hết hạn hoặc chưa đăng nhập</p>
      <button onClick=${onSuccess}
        style="width:100%;padding:12px;background:var(--grad1);color:#fff;border:none;border-radius:8px;font-size:14px;font-weight:600;cursor:pointer">
        🔓 Thử lại
      </button>
    </div>
  </div>`;
}

// ═══ MAIN APP ═══
export function App() {
  const initPage = location.pathname.replace(/^\//, '').replace(/\/$/, '') || 'dashboard';
  const [currentPage, setCurrentPage] = useState(initPage);
  const [lang, setLang] = useState(localStorage.getItem('bizclaw_lang') || 'vi');
  const [wsStatus, setWsStatus] = useState('disconnected');
  const [config, setConfig] = useState({});
  const [toast, setToast] = useState(null);
  const [paired, setPaired] = useState(false);
  const [checkingPairing, setCheckingPairing] = useState(true);
  const [theme, setTheme] = useState(localStorage.getItem('bizclaw_theme') || 'dark');
  const [showOnboarding, setShowOnboarding] = useState(!localStorage.getItem('bizclaw_onboarded'));
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [smeMode, setSmeMode] = useState(localStorage.getItem('bizclaw_sme_mode') === 'true');

  // Apply theme
  useEffect(() => {
    document.documentElement.classList.toggle('light', theme === 'light');
  }, [theme]);
  const wsRef = useRef(null);

  // Check auth
  useEffect(() => {
    (async () => {
      try {
        const verifyBody = jwtToken ? { token: jwtToken } : {};
        const res = await fetch('/api/v1/verify-pairing', {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(verifyBody)
        });
        const r = await res.json();
        if (r.ok) {
          setPaired(true);
        } else {
          sessionStorage.removeItem('bizclaw_jwt');
          jwtToken = '';
          setToken('');
        }
      } catch (e) { setPaired(true); }
      setCheckingPairing(false);
    })();
  }, []);

  // Load config
  useEffect(() => {
    if (!paired) return;
    (async () => {
      try {
        const res = await authFetch('/api/v1/config');
        const data = await res.json();
        setConfig(data);
      } catch (e) { console.error('Config load:', e); }
    })();
  }, [paired]);

  // WebSocket
  useEffect(() => {
    let cancelled = false;
    let reconnectAttempts = 0;
    let pingTimer = null;
    let reconnectTimer = null;

    function connect() {
      if (cancelled) return;
      const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
      let authParam = '';
      if (jwtToken) authParam = '?token=' + encodeURIComponent(jwtToken);
      const url = proto + '//' + location.host + '/ws' + authParam;

      try {
        const socket = new WebSocket(url);
        socket.onopen = () => {
          if (cancelled) { socket.close(); return; }
          reconnectAttempts = 0;
          setWsStatus('connected');
          pingTimer = setInterval(() => {
            if (socket.readyState === 1) socket.send(JSON.stringify({ type: 'ping' }));
          }, 25000);
        };
        socket.onclose = (ev) => {
          setWsStatus('disconnected');
          if (pingTimer) { clearInterval(pingTimer); pingTimer = null; }
          if (!cancelled) {
            reconnectAttempts++;
            const delay = Math.min(1000 * Math.pow(1.5, reconnectAttempts), 30000);
            console.log(`[WS] Closed (code=${ev.code}). Reconnecting in ${Math.round(delay/1000)}s (#${reconnectAttempts})`);
            window.dispatchEvent(new CustomEvent('ws-message', { detail: {
              type: 'error',
              message: `Mất kết nối. Thử lại lần ${reconnectAttempts} trong ${Math.round(delay/1000)}s...`
            }}));
            reconnectTimer = setTimeout(connect, delay);
          }
        };
        socket.onerror = (err) => { console.warn('[WS] Error:', err); };
        socket.onmessage = (e) => {
          try {
            const msg = JSON.parse(e.data);
            window.dispatchEvent(new CustomEvent('ws-message', { detail: msg }));
          } catch (err) {}
        };
        wsRef.current = socket;
        window._ws = socket;
      } catch (e) {
        console.warn('[WS] Failed:', e);
        if (!cancelled) reconnectTimer = setTimeout(connect, 2000);
      }
    }
    reconnectTimer = setTimeout(connect, 500);

    return () => {
      cancelled = true;
      if (reconnectTimer) clearTimeout(reconnectTimer);
      if (pingTimer) clearInterval(pingTimer);
      if (wsRef.current) {
        wsRef.current.onclose = null;
        wsRef.current.close();
      }
    };
  }, []);

  // Browser back/forward
  useEffect(() => {
    const handlePop = () => {
      const p = location.pathname.replace(/^\//, '').replace(/\/$/, '') || 'dashboard';
      setCurrentPage(p);
    };
    window.addEventListener('popstate', handlePop);
    return () => window.removeEventListener('popstate', handlePop);
  }, []);

  const changeLang = useCallback((l) => {
    setLang(l);
    localStorage.setItem('bizclaw_lang', l);
  }, []);

  const showToast = useCallback((msg, type = 'info') => {
    setToast({ message: msg, type });
    setTimeout(() => setToast(null), 3000);
  }, []);
  window.showToast = showToast;

  const navigate = useCallback((pageId) => {
    const path = '/' + (pageId === 'dashboard' ? '' : pageId);
    if (location.pathname !== path) {
      history.pushState({}, '', path);
    }
    setCurrentPage(pageId);
  }, []);

  // Global refs
  window._navigate = navigate;
  window._changeLang = changeLang;
  window._toggleTheme = () => {
    const next = theme === 'dark' ? 'light' : 'dark';
    setTheme(next);
    localStorage.setItem('bizclaw_theme', next);
  };
  window._closeMobileMenu = () => setMobileMenuOpen(false);
  window._toggleSmeMode = (enabled) => {
    setSmeMode(enabled);
    localStorage.setItem('bizclaw_sme_mode', enabled ? 'true' : 'false');
    if (enabled) {
      navigate('sme');
    }
  };

  // Global click handler
  useEffect(() => {
    const handler = (e) => {
      const link = e.target.closest('a[data-page]');
      if (link) {
        e.preventDefault();
        e.stopPropagation();
        const pageId = link.getAttribute('data-page');
        if (pageId && window._navigate) {
          window._navigate(pageId);
          if (window._closeMobileMenu) window._closeMobileMenu();
        }
        return;
      }
      const langBtn = e.target.closest('button[data-lang]');
      if (langBtn) {
        const l = langBtn.getAttribute('data-lang');
        if (l && window._changeLang) window._changeLang(l);
        return;
      }
      const themeBtn = e.target.closest('[data-theme-toggle]');
      if (themeBtn) {
        if (window._toggleTheme) window._toggleTheme();
        return;
      }
    };
    document.addEventListener('click', handler, true);
    return () => document.removeEventListener('click', handler, true);
  }, []);

  // Card radial glow
  useEffect(() => {
    const handler = (e) => {
      const cards = document.querySelectorAll('.card');
      cards.forEach(card => {
        const rect = card.getBoundingClientRect();
        card.style.setProperty('--mouse-x', (e.clientX - rect.left) + 'px');
        card.style.setProperty('--mouse-y', (e.clientY - rect.top) + 'px');
      });
    };
    document.addEventListener('mousemove', handler);
    return () => document.removeEventListener('mousemove', handler);
  }, []);

  // Load ChatWidget lazily
  const [ChatWidget, setChatWidget] = useState(null);
  useEffect(() => {
    import('/static/dashboard/pages/chat_widget.js').then(mod => {
      setChatWidget(() => mod.ChatWidget);
    }).catch(e => console.warn('ChatWidget load failed:', e));
  }, []);

  // Early returns AFTER all hooks
  if (checkingPairing) return html`<div style="display:flex;align-items:center;justify-content:center;height:100vh;background:var(--bg);color:var(--text2)">⏳ Loading...</div>`;
  if (!paired) return html`<${AuthGate} onSuccess=${() => setPaired(true)} />`;

  // ── Welcome Onboarding — first-time user experience ──
  if (showOnboarding) {
    return html`<${OnboardingWizard}
      lang=${lang}
      onComplete=${() => setShowOnboarding(false)}
    />`;
  }

  // ── SME Mode Layout ──
  if (smeMode) {
    return html`
      <${AppContext.Provider} value=${{ config, lang, t: (k) => t(k, lang), showToast, navigate, wsStatus }}>
        <div class="app sme-mode">
          <${SmeTopBar}
            currentPage=${currentPage}
            lang=${lang}
            agentName=${config?.agent_name || 'BizClaw Agent'}
            onNavigate=${navigate}
            smeMode=${smeMode}
            onToggleSme=${(enabled) => {
              setSmeMode(enabled);
              localStorage.setItem('bizclaw_sme_mode', enabled ? 'true' : 'false');
              if (!enabled) {
                navigate('dashboard');
              }
            }}
          />
          <main class="main" style="height:calc(100vh - var(--menu-height));overflow-y:auto">
            <${PageRouter} key=${currentPage} page=${currentPage} config=${config} lang=${lang} />
          </main>
        </div>
        <${Toast} ...${toast || {}} />
      <//>
    `;
  }

  // ── Normal Layout (Developer Mode) ──
  return html`
    <${AppContext.Provider} value=${{ config, lang, t: (k) => t(k, lang), showToast, navigate, wsStatus }}>
      <div class="app">
        <${Sidebar}
          currentPage=${currentPage}
          lang=${lang}
          wsStatus=${wsStatus}
          agentName=${config?.agent_name || 'BizClaw Agent'}
          theme=${theme}
          isOpen=${mobileMenuOpen}
          onClose=${() => setMobileMenuOpen(false)}
          smeMode=${smeMode}
          onToggleSme=${(enabled) => {
            setSmeMode(enabled);
            localStorage.setItem('bizclaw_sme_mode', enabled ? 'true' : 'false');
            if (enabled) {
              navigate('sme');
            }
          }}
        />
        <main class="main">
          <div class="mobile-header">
            <div class="mobile-logo">
              <span class="icon">⚡</span>
              <span class="text">BizClaw</span>
            </div>
            <button class="mobile-hamburger" onClick=${() => setMobileMenuOpen(true)}>☰</button>
          </div>
          <${PageRouter} key=${currentPage} page=${currentPage} config=${config} lang=${lang} />
        </main>
      </div>
      <${Toast} ...${toast || {}} />
      ${ChatWidget ? html`<${ChatWidget} />` : null}
    <//>
  `;
}
