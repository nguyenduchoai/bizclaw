// BizClaw Dashboard — New App with Horizontal Menu
// Preact + HTM, no build step required

const { h, html, render, createContext,
        useState, useEffect, useContext, useCallback, useRef, useMemo } = window;

import { t, authFetch, authHeaders, Toast, StatsCard, PAGES_NEW, getToken, setToken, refreshJwtToken } from '/static/dashboard/shared.js';
import { OnboardingWizard } from '/static/dashboard/pages/onboarding.js';

// ═══ APP CONTEXT ═══
const AppContext = createContext({});
export function useApp() { return useContext(AppContext); }
window.AppContext = AppContext;

let jwtToken = getToken();

// ═══ LAZY PAGE LOADER ═══
const pageCache = {};

async function loadPage(pageId) {
  if (pageCache[pageId]) return pageCache[pageId];

  const PAGE_MAP = {
    dashboard:     { file: 'dashboard.js',        export: 'DashboardPage' },
    chat:          { file: 'chat.js',             export: 'ChatPage' },
    hands:         { file: 'hands.js',            export: 'HandsPage' },
    settings:      { file: 'settings.js',         export: 'SettingsPage' },
    providers:     { file: 'providers.js',         export: 'ProvidersPage' },
    channels:      { file: 'channels.js',          export: 'ChannelsPage' },
    tools:         { file: 'tools.js',             export: 'ToolsPage' },
    mcp:           { file: 'mcp.js',               export: 'McpPage' },
    agents:        { file: 'agents.js?v=2',        export: 'AgentsPage' },
    knowledge:     { file: 'knowledge.js',         export: 'KnowledgePage' },
    kanban:        { file: 'kanban.js',            export: 'KanbanPage' },
    brain:         { file: 'settings.js',          export: 'SettingsPage' },
    configfile:    { file: 'config_file.js',       export: 'ConfigFilePage' },
    scheduler:     { file: 'scheduler.js',          export: 'SchedulerPage' },
    cost:          { file: 'cost.js',              export: 'CostPage' },
    activity:      { file: 'activity.js',           export: 'ActivityPage' },
    workflows:     { file: 'workflows.js',          export: 'WorkflowsPage' },
    wiki:          { file: 'wiki.js',              export: 'WikiPage' },
    apikeys:       { file: 'api_keys.js',          export: 'ApiKeysPage' },
    usage:         { file: 'usage.js',             export: 'UsagePage' },
    analytics:     { file: 'analytics.js',          export: 'AnalyticsPage' },
    plugins:       { file: 'plugins.js',            export: 'PluginsPage' },
    gallery:       { file: 'gallery.js',            export: 'GalleryPage' },
    dbassistant:   { file: 'db_assistant.js',       export: 'DbAssistantPage' },
    campaigns:     { file: 'campaigns.js?v=2',      export: 'CampaignsPage' },
    products:      { file: 'products.js',           export: 'ProductsPage' },
    tiktok:        { file: 'tiktok.js',              export: 'TikTokPage' },
    shopee:        { file: 'shopee.js',              export: 'ShopeePage' },
    handoff:       { file: 'handoff.js?v=2',        export: 'HandoffPage' },
    paymentlinks:  { file: 'payment_links.js',      export: 'PaymentLinksPage' },
    cloud:         { file: 'cloud.js',              export: 'CloudPage' },
    skills:        { file: 'skills.js',             export: 'SkillsPage' },
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

// ═══ PAGE ROUTER ═══
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
    return html`<div style="display:flex;align-items:center;justify-content:center;padding:60px;color:var(--text-secondary)">
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

// ═══ HORIZONTAL TOP NAVIGATION ═══
function TopNav({ currentPage, lang, wsStatus, agentName, theme, onNavigate, onThemeToggle, onLangChange }) {
  const [mobileOpen, setMobileOpen] = useState(false);

  const navGroups = PAGES_NEW;

  return html`
    <nav class="topnav">
      <div class="topnav-main">
        <!-- Logo -->
        <a href="/" class="logo" onClick=${(e) => { e.preventDefault(); onNavigate('dashboard'); }}>
          <span class="icon">⚡</span>
          <span class="text">BizClaw</span>
        </a>

        <!-- Nav Groups -->
        ${navGroups.map(group => html`
          <div key=${group.id} class="nav-group" role="group">
            ${group.children.length === 1 ? html`
              <a href="/${group.children[0].id === 'dashboard' ? '' : group.children[0].id}"
                 class="nav-item ${currentPage === group.children[0].id ? 'active' : ''}"
                 onClick=${(e) => { e.preventDefault(); onNavigate(group.children[0].id); }}>
                ${group.children[0].icon ? html`<span>${group.children[0].icon}</span>` : null}
                <span>${group.children[0].label}</span>
              </a>
            ` : html`
              <div class="nav-dropdown">
                <button class="nav-item ${group.children.some(c => currentPage === c.id) ? 'active' : ''}">
                  ${group.icon ? html`<span>${group.icon}</span>` : null}
                  <span>${group.label}</span>
                  <svg width="12" height="12" viewBox="0 0 12 12" fill="none" style="margin-left:2px;opacity:0.6">
                    <path d="M3 4.5L6 7.5L9 4.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                  </svg>
                </button>
                <div class="nav-dropdown-menu">
                  ${group.children.map(child => html`
                    <a key=${child.id} href="/${child.id === 'dashboard' ? '' : child.id}"
                       class="nav-dropdown-item ${currentPage === child.id ? 'active' : ''}"
                       onClick=${(e) => { e.preventDefault(); onNavigate(child.id); setMobileOpen(false); }}>
                      ${child.icon ? html`<span class="icon">${child.icon}</span>` : null}
                      <span>${child.label}</span>
                      ${child.badge ? html`<span class="badge badge-${child.badge}">${child.badge}</span>` : null}
                    </a>
                  `)}
                </div>
              </div>
            `}
          </div>
        `)}

        <!-- Right Side -->
        <div class="topnav-right">
          <div class="ws-indicator">
            <span class="ws-dot ${wsStatus === 'connected' ? 'online' : wsStatus === 'connecting' ? 'connecting' : 'offline'}"></span>
            <span>${wsStatus === 'connected' ? t('status.connected', lang) : t('status.disconnected', lang)}</span>
          </div>

          <button class="theme-toggle" onClick=${onThemeToggle}>
            ${theme === 'light' ? '🌙' : '☀️'} ${theme === 'light' ? 'Dark' : 'Light'}
          </button>

          <div class="lang-switch">
            <button class="lang-btn ${lang === 'vi' ? 'active' : ''}"
              onClick=${() => onLangChange('vi')}>VI</button>
            <button class="lang-btn ${lang === 'en' ? 'active' : ''}"
              onClick=${() => onLangChange('en')}>EN</button>
          </div>

          <button class="profile-btn">
            <div class="profile-avatar">${(agentName || 'B').charAt(0).toUpperCase()}</div>
            <span class="profile-name">${agentName || 'Agent'}</span>
          </button>

          <button class="mobile-menu-btn" onClick=${() => setMobileOpen(!mobileOpen)}>
            ${mobileOpen ? '✕' : '☰'}
          </button>
        </div>
      </div>
    </nav>

    <!-- Mobile Sidebar -->
    <div class="mobile-sidebar ${mobileOpen ? 'open' : ''}" onClick=${() => setMobileOpen(false)}>
      <div class="mobile-sidebar-panel" onClick=${(e) => e.stopPropagation()}>
        <div class="mobile-sidebar-header">
          <span class="logo">
            <span class="icon">⚡</span>
            <span class="text">BizClaw</span>
          </span>
          <button onClick=${() => setMobileOpen(false)} style="background:none;border:none;font-size:20px;cursor:pointer">✕</button>
        </div>
        <div class="mobile-sidebar-nav">
          ${navGroups.map(group => html`
            <div key=${group.id} style="margin-bottom:16px">
              <div style="font-size:10px;font-weight:700;text-transform:uppercase;letter-spacing:1px;color:var(--text-secondary);padding:8px 0;opacity:0.7">
                ${group.label}
              </div>
              ${group.children.map(child => html`
                <a key=${child.id} href="/${child.id === 'dashboard' ? '' : child.id}"
                   class="nav-dropdown-item ${currentPage === child.id ? 'active' : ''}"
                   onClick=${(e) => { e.preventDefault(); onNavigate(child.id); setMobileOpen(false); }}>
                  ${child.icon ? html`<span class="icon">${child.icon}</span>` : null}
                  <span>${child.label}</span>
                </a>
              `)}
            </div>
          `)}
        </div>
      </div>
    </div>

    <!-- Mobile Bottom Nav -->
    <div class="mobile-nav">
      <div class="mobile-nav-items">
        ${['dashboard', 'agents', 'chat', 'workflows', 'settings'].map(id => {
          const page = PAGES_NEW.flatMap(g => g.children).find(c => c.id === id);
          if (!page) return null;
          return html`
            <button key=${id} class="mobile-nav-item ${currentPage === id ? 'active' : ''}"
              onClick=${() => onNavigate(id)}>
              <span class="icon">${page.icon}</span>
              <span>${page.label}</span>
            </button>
          `;
        })}
      </div>
    </div>
  `;
}

// ═══ AUTH GATE ═══
function AuthGate({ onSuccess }) {
  return html`<div style="position:fixed;inset:0;background:var(--bg-secondary);z-index:300;display:flex;align-items:center;justify-content:center">
    <div style="background:var(--bg-card);border:1px solid var(--border);border-radius:16px;padding:40px;width:380px;text-align:center">
      <div style="font-size:32px;margin-bottom:12px">🔐</div>
      <h2 style="color:var(--accent);margin-bottom:8px">BizClaw Agent</h2>
      <p style="color:var(--text-secondary);font-size:13px;margin-bottom:24px">Phiên đăng nhập hết hạn hoặc chưa đăng nhập</p>
      <button onClick=${onSuccess}
        style="width:100%;padding:12px;background:var(--accent);color:#fff;border:none;border-radius:8px;font-size:14px;font-weight:600;cursor:pointer">
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
  const [theme, setTheme] = useState('light');
  const [showOnboarding, setShowOnboarding] = useState(!localStorage.getItem('bizclaw_onboarded'));
  const wsRef = useRef(null);

  // Apply theme
  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme);
  }, [theme]);

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

  window._navigate = navigate;
  window._changeLang = changeLang;
  window._toggleTheme = () => {
    const next = theme === 'dark' ? 'light' : 'dark';
    setTheme(next);
    localStorage.setItem('bizclaw_theme', next);
  };

  // Global click handler
  useEffect(() => {
    const handler = (e) => {
      const link = e.target.closest('a[data-page]');
      if (link) {
        e.preventDefault();
        const pageId = link.getAttribute('data-page');
        if (pageId && window._navigate) window._navigate(pageId);
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

  // Load ChatWidget lazily
  const [ChatWidget, setChatWidget] = useState(null);
  useEffect(() => {
    import('/static/dashboard/pages/chat_widget.js').then(mod => {
      setChatWidget(() => mod.ChatWidget);
    }).catch(e => console.warn('ChatWidget load failed:', e));
  }, []);

  if (checkingPairing) return html`<div style="display:flex;align-items:center;justify-content:center;height:100vh;background:var(--bg-secondary);color:var(--text-secondary)">⏳ Loading...</div>`;
  if (!paired) return html`<${AuthGate} onSuccess=${() => setPaired(true)} />`;

  if (showOnboarding) {
    return html`<${OnboardingWizard}
      lang=${lang}
      onComplete=${() => setShowOnboarding(false)}
    />`;
  }

  return html`
    <${AppContext.Provider} value=${{ config, lang, t: (k) => t(k, lang), showToast, navigate, wsStatus }}>
      <${TopNav}
        currentPage=${currentPage}
        lang=${lang}
        wsStatus=${wsStatus}
        agentName=${config?.agent_name || 'BizClaw Agent'}
        theme=${theme}
        onNavigate=${navigate}
        onThemeToggle=${() => window._toggleTheme()}
        onLangChange=${changeLang}
      />
      <div class="main-wrapper">
        <div class="main-content">
          <${PageRouter} key=${currentPage} page=${currentPage} config=${config} lang=${lang} />
        </div>
      </div>
      <${Toast} ...${toast || {}} />
      ${ChatWidget ? html`<${ChatWidget} />` : null}
    <//>
  `;
}