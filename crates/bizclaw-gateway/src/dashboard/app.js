// BizClaw Dashboard — Main App Component
// Preact + HTM, no build step required
//
// CRITICAL: Do NOT import preact/hooks/htm here!
// esm.sh CDN creates SEPARATE Preact instances for 'preact' and 'preact/hooks'
// (the dual-package hazard). If app.js imports its own copy, useState/setPage
// register with Preact Instance B while render() uses Instance A → no re-renders.
//
// Solution: Use window.* globals set by index.html from a single import chain.
const { h, html, render, createContext,
        useState, useEffect, useContext, useCallback, useRef, useMemo } = window;

import { vi } from '/static/dashboard/i18n/vi.js';
import { en } from '/static/dashboard/i18n/en.js';

const I18N = { vi, en };

// ═══ APP CONTEXT ═══
const AppContext = createContext({});

export function useApp() { return useContext(AppContext); }

// ═══ API HELPERS ═══
let pairingCode = sessionStorage.getItem('bizclaw_pairing') || '';

function authHeaders(extra = {}) {
  return { ...extra, 'X-Pairing-Code': pairingCode, 'Content-Type': 'application/json' };
}

async function authFetch(url, opts = {}) {
  if (!opts.headers) opts.headers = {};
  opts.headers['X-Pairing-Code'] = pairingCode;
  const res = await fetch(url, opts);
  if (res.status === 401) {
    sessionStorage.removeItem('bizclaw_pairing');
    pairingCode = '';
    throw new Error('Invalid pairing code');
  }
  return res;
}

// Export for page modules
window.authFetch = authFetch;
window.authHeaders = authHeaders;

// ═══ I18N ═══
function t(key, lang) {
  return (I18N[lang] || I18N.vi)[key] || I18N.vi[key] || key;
}

// ═══ PAGES (lazy loaded) ═══
const PAGES = [
  { id: 'dashboard', icon: '📊', label: 'nav.dashboard' },
  { id: 'chat', icon: '💬', label: 'nav.webchat' },
  { id: 'sep1', sep: true },
  { id: 'agents', icon: '🤖', label: 'nav.agents' },
  { id: 'knowledge', icon: '📚', label: 'nav.knowledge' },
  { id: 'channels', icon: '📱', label: 'nav.channels' },
  { id: 'settings', icon: '⚙️', label: 'nav.settings' },
  { id: 'providers', icon: '🔌', label: 'nav.providers' },
  { id: 'tools', icon: '🛠️', label: 'nav.tools' },
  { id: 'mcp', icon: '🔗', label: 'nav.mcp' },
  { id: 'wiki', icon: '📖', label: 'Wiki & Guide' },
  { id: 'sep2', sep: true },
  { id: 'hands', icon: '🤚', label: 'Autonomous Hands' },
  { id: 'workflows', icon: '🔄', label: 'nav.workflows' },
  { id: 'skills', icon: '🧩', label: 'nav.skills' },
  { id: 'orchestration', icon: '🔀', label: 'nav.orchestration' },
  { id: 'gallery', icon: '📦', label: 'nav.gallery' },
  { id: 'scheduler', icon: '⏰', label: 'nav.scheduler' },
  { id: 'traces', icon: '📊', label: 'LLM Traces' },
  { id: 'cost', icon: '💰', label: 'Cost Tracking' },
  { id: 'activity', icon: '⚡', label: 'Activity Feed' },
  { id: 'sep3', sep: true },
  { id: 'configfile', icon: '📄', label: 'nav.config' },
];

// ═══ TOAST ═══
function Toast({ message, type }) {
  if (!message) return null;
  const colors = { error: 'var(--red)', success: 'var(--green)', info: 'var(--accent2)' };
  return html`<div class="toast" style="border-left: 3px solid ${colors[type] || colors.info}">
    ${message}
  </div>`;
}

// ═══ STATS CARD ═══
function StatsCard({ label, value, color = 'accent', sub, icon }) {
  return html`<div class="card" style="text-align:center">
    <div class="card-label">${icon ? icon + ' ' : ''}${label}</div>
    <div class="card-value ${color}" style="font-size:${String(value).length > 8 ? '18' : '26'}px">${value}</div>
    ${sub && html`<div class="card-sub">${sub}</div>`}
  </div>`;
}
window.StatsCard = StatsCard;

// ═══ SIDEBAR ═══
function Sidebar({ currentPage, onNavigate, lang, onLangChange, wsStatus, agentName }) {
  return html`<aside class="sidebar">
    <div class="logo">
      <span class="icon">⚡</span>
      <span class="text">BizClaw</span>
    </div>
    <nav class="nav">
      ${PAGES.map(p => p.sep
        ? html`<div class="nav-sep" key=${p.id}></div>`
        : html`<a key=${p.id} href="/${p.id === 'dashboard' ? '' : p.id}"
              class=${currentPage === p.id ? 'active' : ''}
              onClick=${(e) => { e.preventDefault(); onNavigate(p.id); }}>
            ${p.icon} <span>${t(p.label, lang)}</span>
          </a>`
      )}
    </nav>
    <div class="sidebar-footer">
      <div style="display:flex;align-items:center;gap:6px;margin-bottom:6px">
        <button onClick=${() => onLangChange('vi')}
          style="padding:2px 8px;font-size:11px;border-radius:4px;border:1px solid var(--border);background:${lang === 'vi' ? 'var(--accent)' : 'transparent'};color:${lang === 'vi' ? '#fff' : 'var(--text2)'};cursor:pointer">VI</button>
        <button onClick=${() => onLangChange('en')}
          style="padding:2px 8px;font-size:11px;border-radius:4px;border:1px solid var(--border);background:${lang === 'en' ? 'var(--accent)' : 'transparent'};color:${lang === 'en' ? '#fff' : 'var(--text2)'};cursor:pointer">EN</button>
      </div>
      <div>${wsStatus === 'connected' ? '🟢' : '🔴'} ${t(wsStatus === 'connected' ? 'status.connected' : 'status.disconnected', lang)}</div>
      <div style="margin-top:4px">${agentName}</div>
    </div>
  </aside>`;
}

// ═══ PAIRING GATE ═══
function PairingGate({ onSuccess }) {
  const [code, setCode] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  const doPairing = async () => {
    setError('');
    if (!code.trim()) { setError('Vui lòng nhập mã pairing'); return; }
    setLoading(true);
    try {
      const res = await fetch('/api/v1/verify-pairing', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ code: code.trim() })
      });
      const r = await res.json();
      if (r.ok) {
        pairingCode = code.trim();
        sessionStorage.setItem('bizclaw_pairing', pairingCode);
        onSuccess();
      } else {
        setError(r.error || 'Sai mã pairing');
      }
    } catch (e) { setError(e.message); }
    setLoading(false);
  };

  return html`<div style="position:fixed;inset:0;background:var(--bg);z-index:300;display:flex;align-items:center;justify-content:center">
    <div style="background:var(--surface);border:1px solid var(--border);border-radius:16px;padding:40px;width:380px;text-align:center">
      <div style="font-size:32px;margin-bottom:12px">🔐</div>
      <h2 style="color:var(--accent);margin-bottom:8px">BizClaw Agent</h2>
      <p style="color:var(--text2);font-size:13px;margin-bottom:24px">Nhập mã Pairing Code để truy cập Dashboard</p>
      ${error && html`<div style="color:var(--red);font-size:13px;margin-bottom:12px">${error}</div>`}
      <input type="text" value=${code} onInput=${e => setCode(e.target.value)}
        placeholder="Pairing Code (6 digits)" maxlength="10"
        onKeyDown=${e => e.key === 'Enter' && doPairing()}
        style="width:100%;padding:12px 16px;margin-bottom:14px;background:var(--bg2);border:1px solid var(--border);border-radius:8px;color:var(--text);font-size:18px;text-align:center;letter-spacing:4px;font-family:var(--mono)" />
      <button onClick=${doPairing} disabled=${loading}
        style="width:100%;padding:12px;background:var(--grad1);color:#fff;border:none;border-radius:8px;font-size:14px;font-weight:600;cursor:pointer">
        ${loading ? '⏳...' : '🔓 Xác nhận'}
      </button>
    </div>
  </div>`;
}

// ═══ CHAT PAGE ═══
function ChatPage({ config, lang }) {
  const [messages, setMessages] = useState([]);
  const [input, setInput] = useState('');
  const [thinking, setThinking] = useState(false);
  const [streamContent, setStreamContent] = useState('');
  const [streamReqId, setStreamReqId] = useState(null);
  const [sessions, setSessions] = useState([{ id: 'main', name: 'Main Chat', icon: '🤖', time: 'now', count: 0 }]);
  const [activeSession, setActiveSession] = useState('main');
  const [wsInfo, setWsInfo] = useState({});
  const messagesEndRef = useRef(null);
  const inputRef = useRef(null);

  // Auto-scroll to bottom
  useEffect(() => {
    if (messagesEndRef.current) {
      messagesEndRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  }, [messages, streamContent]);

  // Listen for WS messages
  useEffect(() => {
    const handler = (e) => {
      const msg = e.detail;
      if (!msg || !msg.type) return;

      switch (msg.type) {
        case 'connected':
          setWsInfo(msg);
          setMessages(prev => [...prev, { type: 'system', content: `${t('chat.welcome', lang)}\n🤖 Provider: ${msg.provider} | Model: ${msg.model}${msg.agent_engine ? ' | 🧠 Agent Engine' : ''}` }]);
          break;

        case 'chat_start':
          setStreamReqId(msg.request_id);
          setStreamContent('');
          setThinking(false);
          break;

        case 'chat_chunk':
          setStreamContent(prev => prev + (msg.content || ''));
          break;

        case 'chat_done': {
          const fullContent = msg.full_content || '';
          setMessages(prev => [...prev, { type: 'bot', content: fullContent, provider: msg.provider, model: msg.model, mode: msg.mode, context: msg.context }]);
          setStreamContent('');
          setStreamReqId(null);
          setThinking(false);
          // Update session count
          setSessions(prev => prev.map(s => s.id === activeSession ? { ...s, count: (s.count || 0) + 1, time: new Date().toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit' }) } : s));
          break;
        }

        case 'chat_response':
          setMessages(prev => [...prev, { type: 'bot', content: msg.content || '', provider: msg.provider, model: msg.model }]);
          setThinking(false);
          break;

        case 'chat_error':
          setMessages(prev => [...prev, { type: 'system', content: '❌ Error: ' + (msg.error || 'Unknown error'), error: true }]);
          setThinking(false);
          setStreamContent('');
          setStreamReqId(null);
          break;

        case 'status':
          setMessages(prev => [...prev, { type: 'system', content: `📊 Status:\n• Provider: ${msg.provider}\n• Model: ${msg.model}\n• Requests: ${msg.requests_processed}\n• Uptime: ${Math.floor(msg.uptime_secs / 60)}m ${msg.uptime_secs % 60}s\n• Agent Engine: ${msg.agent_engine ? '✅ Active' : '❌ Off'}` }]);
          break;

        case 'pong':
          break; // silent

        case 'error':
          setMessages(prev => [...prev, { type: 'system', content: '⚠️ ' + (msg.message || ''), error: true }]);
          break;
      }
    };

    window.addEventListener('ws-message', handler);
    return () => window.removeEventListener('ws-message', handler);
  }, [lang, activeSession]);

  const sendMessage = () => {
    const text = input.trim();
    if (!text) return;
    setInput('');

    // Handle slash commands locally
    if (text === '/help') {
      setMessages(prev => [...prev, { type: 'system', content: t('chat.help', lang) }]);
      return;
    }
    if (text === '/reset') {
      setMessages([{ type: 'system', content: t('chat.history_cleared', lang) }]);
      return;
    }
    if (text === '/status') {
      if (window._ws && window._ws.readyState === 1) {
        window._ws.send(JSON.stringify({ type: 'status' }));
      }
      return;
    }
    if (text === '/export') {
      const chatText = messages.map(m => {
        if (m.type === 'user') return `You: ${m.content}`;
        if (m.type === 'bot') return `AI: ${m.content}`;
        return `[${m.content}]`;
      }).join('\n\n');
      const blob = new Blob([chatText], { type: 'text/plain' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url; a.download = `bizclaw-chat-${Date.now()}.txt`; a.click();
      URL.revokeObjectURL(url);
      setMessages(prev => [...prev, { type: 'system', content: '📄 Chat exported!' }]);
      return;
    }

    // Add user message to UI
    setMessages(prev => [...prev, { type: 'user', content: text }]);
    setThinking(true);

    // Send via WebSocket
    if (window._ws && window._ws.readyState === 1) {
      window._ws.send(JSON.stringify({ type: 'chat', content: text, stream: true }));
    } else {
      setMessages(prev => [...prev, { type: 'system', content: '🔴 WebSocket not connected. Reconnecting...', error: true }]);
      setThinking(false);
    }
  };

  // Render markdown-ish content (code blocks, bold, links)
  const renderContent = (text) => {
    if (!text) return '';
    // Split by code blocks
    const parts = text.split(/(```[\s\S]*?```)/g);
    return parts.map((part, i) => {
      if (part.startsWith('```') && part.endsWith('```')) {
        const inner = part.slice(3, -3);
        const firstLine = inner.indexOf('\n');
        const lang = firstLine > 0 ? inner.slice(0, firstLine).trim() : '';
        const code = firstLine > 0 ? inner.slice(firstLine + 1) : inner;
        return html`<div key=${i} style="background:var(--bg);border:1px solid var(--border);border-radius:6px;margin:6px 0;overflow-x:auto">
          ${lang && html`<div style="padding:4px 10px;font-size:10px;color:var(--text2);border-bottom:1px solid var(--border);text-transform:uppercase">${lang}</div>`}
          <pre style="padding:10px 14px;font-size:12px;font-family:var(--mono);white-space:pre-wrap;word-break:break-all;margin:0;color:var(--cyan)">${code}</pre>
        </div>`;
      }
      // Inline formatting: bold
      return html`<span key=${i} dangerouslySetInnerHTML=${{ __html: part.replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>').replace(/\n/g, '<br/>') }} />`;
    });
  };

  const fmtTime = () => new Date().toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit' });

  return html`<div style="height:calc(100vh - 56px);display:flex;flex-direction:column">
    <div class="chat-layout" style="flex:1;height:100%">
      <!-- Sidebar: conversation list -->
      <div class="chat-sidebar">
        <div class="chat-sidebar-header">
          <h3>💬 ${t('chat.title', lang)}</h3>
          <button class="btn btn-outline btn-sm" onClick=${() => {
            const id = 'chat_' + Date.now();
            setSessions(prev => [{ id, name: 'New Chat', icon: '💬', time: fmtTime(), count: 0 }, ...prev]);
            setActiveSession(id);
            setMessages([]);
          }}>+ New</button>
        </div>
        <div class="chat-list">
          <div class="chat-list-sep">Sessions</div>
          ${sessions.map(s => html`
            <div key=${s.id} class="chat-list-item ${activeSession === s.id ? 'active' : ''}" onClick=${() => setActiveSession(s.id)}>
              <div class="chat-list-icon">${s.icon}</div>
              <div class="chat-list-info">
                <div class="chat-list-name">${s.name}</div>
                <div class="chat-list-sub">${s.count || 0} messages · ${s.time}</div>
              </div>
            </div>
          `)}
          <div class="chat-list-sep" style="margin-top:12px">Commands</div>
          ${[{ cmd: '/help', desc: 'Show help', icon: '❓' }, { cmd: '/status', desc: 'Agent status', icon: '📊' }, { cmd: '/reset', desc: 'Clear history', icon: '🗑️' }, { cmd: '/export', desc: 'Export chat', icon: '📄' }].map(c => html`
            <div key=${c.cmd} class="chat-list-item" onClick=${() => { setInput(c.cmd); if (inputRef.current) inputRef.current.focus(); }}>
              <div class="chat-list-icon" style="font-size:16px">${c.icon}</div>
              <div class="chat-list-info">
                <div class="chat-list-name" style="font-family:var(--mono);font-size:12px">${c.cmd}</div>
                <div class="chat-list-sub">${c.desc}</div>
              </div>
            </div>
          `)}
        </div>
      </div>

      <!-- Main chat area -->
      <div class="chat-main">
        <div class="chat-main-header">
          <div class="chat-target">
            <span class="chat-target-icon">🤖</span>
            <div>
              <div class="chat-target-name">${config?.agent_name || 'BizClaw AI'}</div>
              <div class="chat-target-sub">${wsInfo.provider || config?.default_provider || '—'} · ${wsInfo.model || '—'}${wsInfo.agent_engine ? ' · 🧠 Agent' : ''}</div>
            </div>
          </div>
          <div style="display:flex;gap:6px;align-items:center">
            <span class="badge ${thinking ? 'badge-yellow pulse' : 'badge-green'}">${thinking ? '⏳ thinking' : '● ready'}</span>
            <button class="btn btn-outline btn-sm" onClick=${() => setMessages([])} title="Clear">🗑️</button>
          </div>
        </div>

        <div class="chat-messages">
          ${messages.length === 0 && !streamContent ? html`
            <div style="flex:1;display:flex;align-items:center;justify-content:center">
              <div style="text-align:center;padding:40px">
                <div style="font-size:56px;margin-bottom:16px">🤖</div>
                <h2 style="font-size:18px;margin-bottom:8px;color:var(--accent2)">${config?.agent_name || 'BizClaw AI'}</h2>
                <p style="color:var(--text2);font-size:13px;max-width:360px;margin:0 auto">${t('chat.welcome', lang)}</p>
                <div style="display:flex;gap:8px;margin-top:20px;justify-content:center;flex-wrap:wrap">
                  ${['Bạn là ai?', 'Giúp tôi viết email', 'Phân tích doanh thu Q4', 'Tạo kế hoạch marketing'].map(q => html`
                    <button key=${q} class="btn btn-outline btn-sm" onClick=${() => { setInput(q); }}>${q}</button>
                  `)}
                </div>
              </div>
            </div>
          ` : html`
            ${messages.map((m, i) => html`
              <div key=${i} class=${m.type === 'user' ? 'msg msg-user' : m.type === 'bot' ? 'msg msg-bot' : 'msg msg-system'}
                style=${m.error ? 'color:var(--red)' : ''}>
                ${m.type === 'bot' ? renderContent(m.content) : m.content}
                ${m.type === 'bot' && m.mode === 'agent' ? html`<div style="font-size:10px;color:var(--text2);margin-top:4px;text-align:right">🧠 Agent${m.context ? ' · ctx:' + m.context.total_tokens : ''}</div>` : ''}
              </div>
            `)}
            ${streamContent ? html`<div class="msg msg-bot">${renderContent(streamContent)}<span class="pulse" style="color:var(--accent2)">▊</span></div>` : ''}
            ${thinking && !streamContent ? html`<div class="typing" style="display:flex;align-items:center;gap:6px">
              <span class="pulse">●</span> ${t('chat.thinking', lang)}...
            </div>` : ''}
            <div ref=${messagesEndRef} />
          `}
        </div>

        <div class="chat-input-wrap">
          <input ref=${inputRef} value=${input} onInput=${e => setInput(e.target.value)}
            onKeyDown=${e => e.key === 'Enter' && !e.shiftKey && sendMessage()}
            placeholder=${t('chat.placeholder', lang)} autocomplete="off" />
          <button onClick=${sendMessage} disabled=${thinking}>${t('chat.send', lang)}</button>
        </div>
      </div>
    </div>
  </div>`;
}

// ═══ DASHBOARD PAGE ═══
function DashboardPage({ config, lang }) {
  const [clock, setClock] = useState('--:--:--');
  const [dateStr, setDateStr] = useState('');

  useEffect(() => {
    const timer = setInterval(() => {
      const now = new Date();
      setClock(now.toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' }));
      setDateStr(now.toLocaleDateString(lang === 'vi' ? 'vi-VN' : 'en-US', { weekday: 'short', month: 'short', day: 'numeric' }));
    }, 1000);
    return () => clearInterval(timer);
  }, [lang]);

  const provider = config?.default_provider || '—';
  const model = config?.providers?.[provider]?.model || '—';
  const version = config?.version || '—';

  return html`<div>
    <div class="page-header"><div>
      <h1>${t('dash.title', lang)}</h1>
      <div class="sub">${t('dash.subtitle', lang)}</div>
    </div></div>

    <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(150px,1fr));gap:12px;margin-bottom:16px">
      <${StatsCard} label=${t('dash.clock', lang)} value=${clock} color="accent" sub=${dateStr} icon="⏰" />
      <${StatsCard} label=${t('dash.uptime', lang)} value=${config?.uptime || '—'} color="green" sub=${t('dash.status', lang)} />
      <${StatsCard} label=${t('dash.provider', lang)} value=${provider} color="blue" sub=${model} />
      <${StatsCard} label=${t('dash.version', lang)} value=${version} color="accent" />
    </div>

    <div style="display:grid;grid-template-columns:1fr 1fr;gap:14px;margin-bottom:16px">
      <div class="card">
        <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:12px">
          <div class="card-label" style="margin:0">🖥️ ${t('dash.system', lang)}</div>
          <span class="badge badge-green">● ${t('dash.online', lang)}</span>
        </div>
        <div style="display:grid;grid-template-columns:1fr 1fr;gap:8px;font-size:12px">
          <div><span style="color:var(--text2)">${t('sys.os', lang)}</span> ${config?.system?.os || '—'}</div>
          <div><span style="color:var(--text2)">${t('sys.arch', lang)}</span> ${config?.system?.arch || '—'}</div>
          <div><span style="color:var(--text2)">SIMD:</span> <span style="color:var(--accent2)">${config?.system?.simd || '—'}</span></div>
          <div><span style="color:var(--text2)">${t('sys.memory', lang)}</span> ${config?.system?.memory || '—'}</div>
        </div>
      </div>
      <div class="card">
        <div class="card-label" style="margin-bottom:10px">⚡ ${t('dash.quickactions', lang)}</div>
        <div style="display:flex;flex-wrap:wrap;gap:6px">
          ${['chat', 'settings', 'channels', 'knowledge', 'configfile'].map(p => html`
            <button class="btn btn-outline btn-sm" key=${p}
              onClick=${() => window._navigate && window._navigate(p)}>
              ${PAGES.find(x => x.id === p)?.icon || ''} ${t(PAGES.find(x => x.id === p)?.label || p, lang)}
            </button>
          `)}
        </div>
      </div>
    </div>
  </div>`;
}

// ═══ SCHEDULER PAGE (with retry UI) ═══
function SchedulerPage({ lang }) {
  const [tasks, setTasks] = useState([]);
  const [stats, setStats] = useState({});
  const [loading, setLoading] = useState(true);
  const [notifications, setNotifications] = useState([]);

  const loadData = async () => {
    try {
      const [tasksRes, notiRes] = await Promise.all([
        authFetch('/api/v1/scheduler/tasks'),
        authFetch('/api/v1/scheduler/notifications'),
      ]);
      const tasksData = await tasksRes.json();
      const notiData = await notiRes.json();
      setTasks(tasksData.tasks || []);
      setStats(tasksData.stats || {});
      setNotifications(notiData.notifications || []);
    } catch (e) { console.error('Scheduler load err:', e); }
    setLoading(false);
  };

  useEffect(() => { loadData(); }, []);

  const toggleTask = async (id, enabled) => {
    await authFetch('/api/v1/scheduler/tasks/' + id + '/toggle', {
      method: 'POST', headers: authHeaders(),
      body: JSON.stringify({ enabled: !enabled })
    });
    loadData();
  };

  const deleteTask = async (id) => {
    if (!confirm('Xóa task này?')) return;
    await authFetch('/api/v1/scheduler/tasks/' + id, { method: 'DELETE', headers: authHeaders() });
    loadData();
  };

  const statusBadge = (status, task) => {
    if (!status) return html`<span class="badge badge-blue">pending</span>`;
    if (status === 'Pending') return html`<span class="badge badge-blue">pending</span>`;
    if (status === 'Running') return html`<span class="badge badge-yellow">running</span>`;
    if (status === 'Completed') return html`<span class="badge badge-green">completed</span>`;
    if (status === 'Disabled') return html`<span class="badge badge-purple">disabled</span>`;
    if (typeof status === 'object' && status.RetryPending)
      return html`<span class="badge badge-orange">🔄 retry ${status.RetryPending.attempt}/${task?.retry?.max_retries || 3}</span>`;
    if (typeof status === 'object' && status.Failed)
      return html`<span class="badge badge-red" title=${status.Failed}>❌ failed</span>`;
    return html`<span class="badge badge-blue">${JSON.stringify(status)}</span>`;
  };

  const taskTypeLabel = (task) => {
    const tt = task.task_type;
    if (!tt) return '—';
    if (tt.Once) return '⏱ Once';
    if (tt.Cron) return '📅 ' + tt.Cron.expression;
    if (tt.Interval) return '🔁 ' + tt.Interval.every_secs + 's';
    return JSON.stringify(tt);
  };

  const formatTime = (t) => {
    if (!t) return '—';
    return new Date(t).toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' });
  };

  const active = tasks.filter(t => t.enabled).length;
  const retrying = tasks.filter(t => t.status && typeof t.status === 'object' && t.status.RetryPending).length;
  const failed = tasks.filter(t => t.status && typeof t.status === 'object' && t.status.Failed && t.fail_count >= (t.retry?.max_retries || 3)).length;

  return html`<div>
    <div class="page-header"><div>
      <h1>⏰ ${t('sched.title', lang)}</h1>
      <div class="sub">${t('sched.subtitle', lang)}</div>
    </div></div>

    <div class="stats">
      <${StatsCard} label="Total Tasks" value=${tasks.length} color="accent" />
      <${StatsCard} label="Active" value=${active} color="green" />
      <${StatsCard} label=${t('sched.retrying', lang)} value=${retrying} color="orange" />
      <${StatsCard} label=${t('sched.failed', lang)} value=${failed} color="red" />
    </div>

    <div class="card">
      <h3 style="margin-bottom:12px">📋 Tasks (${tasks.length})</h3>
      ${loading ? html`<div style="color:var(--text2);text-align:center;padding:20px">Loading...</div>` : html`
        <table>
          <thead><tr>
            <th>Task</th><th>Type</th><th>Action</th><th>Status</th>
            <th>Retries</th><th>Next Run</th><th>Error</th><th></th>
          </tr></thead>
          <tbody>
            ${tasks.map(task => html`<tr key=${task.id}>
              <td><strong>${task.name}</strong></td>
              <td>${taskTypeLabel(task)}</td>
              <td style="font-size:12px">${task.action?.AgentPrompt ? '🤖 Agent' : task.action?.Webhook ? '🌐 Webhook' : '📢 Notify'}</td>
              <td>${statusBadge(task.status, task)}</td>
              <td style="font-family:var(--mono);font-size:12px">${task.fail_count || 0}/${task.retry?.max_retries || 3}</td>
              <td style="font-family:var(--mono);font-size:12px">${formatTime(task.next_run)}</td>
              <td style="max-width:150px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-size:11px;color:var(--red)" title=${task.last_error || ''}>
                ${task.last_error ? task.last_error.substring(0, 50) : '—'}
              </td>
              <td style="white-space:nowrap">
                <button class="btn btn-outline btn-sm" onClick=${() => toggleTask(task.id, task.enabled)}>
                  ${task.enabled ? '⏸' : '▶'}
                </button>
                <button class="btn btn-sm" style="background:var(--red);color:#fff;margin-left:4px" onClick=${() => deleteTask(task.id)}>🗑</button>
              </td>
            </tr>`)}
          </tbody>
        </table>
      `}
    </div>

    ${notifications.length > 0 && html`
      <div class="card" style="margin-top:16px">
        <h3 style="margin-bottom:12px">📨 Notification History (${notifications.length})</h3>
        <table>
          <thead><tr><th>Title</th><th>Priority</th><th>Source</th><th>Time</th></tr></thead>
          <tbody>
            ${notifications.slice(0, 20).map(n => html`<tr key=${n.id}>
              <td>${n.title}</td>
              <td><span class="badge ${n.priority === 'urgent' ? 'badge-red' : n.priority === 'high' ? 'badge-orange' : 'badge-blue'}">${n.priority}</span></td>
              <td style="font-size:12px">${n.source}</td>
              <td style="font-family:var(--mono);font-size:12px">${formatTime(n.created_at)}</td>
            </tr>`)}
          </tbody>
        </table>
      </div>
    `}
  </div>`;
}

// ═══ AUTONOMOUS HANDS PAGE (Full CRUD + API) ═══
function HandsPage({ lang }) {
  const { showToast } = useContext(AppContext);
  const [hands, setHands] = useState([]);
  const [loading, setLoading] = useState(true);
  const [showForm, setShowForm] = useState(false);
  const [editHand, setEditHand] = useState(null);
  const [form, setForm] = useState({name:'',schedule:'',prompt:'',phases:'',icon:'🤚'});

  const defaultHands = [
    { id:'research', name:'Research Hand', icon:'🔍', schedule:'0 */6 * * *', prompt:'Research and gather information on specified topics, analyze findings, produce summary reports.', phases:'gather,analyze,report', enabled:true, runs:0, tokens:0, cost:0 },
    { id:'analytics', name:'Analytics Hand', icon:'📊', schedule:'0 6 * * *', prompt:'Collect metrics and analytics data, process trends, generate daily insight reports.', phases:'collect,process,report', enabled:true, runs:0, tokens:0, cost:0 },
    { id:'content', name:'Content Hand', icon:'📝', schedule:'0 8 * * *', prompt:'Generate content ideas, create drafts, self-review with quality checks.', phases:'ideate,create,review', enabled:true, runs:0, tokens:0, cost:0 },
    { id:'monitor', name:'Monitor Hand', icon:'🔔', schedule:'*/5 * * * *', prompt:'Monitor system health, external services, and alert on anomalies.', phases:'check,alert', enabled:true, runs:0, tokens:0, cost:0 },
    { id:'sync', name:'Sync Hand', icon:'🔄', schedule:'*/30 * * * *', prompt:'Synchronize data between systems, reconcile differences, push updates.', phases:'fetch,reconcile,push', enabled:true, runs:0, tokens:0, cost:0 },
    { id:'outreach', name:'Outreach Hand', icon:'📧', schedule:'0 9 * * 1-5', prompt:'Prepare outreach messages, review content quality, send to configured channels.', phases:'prepare,review,send', enabled:true, runs:0, tokens:0, cost:0 },
    { id:'security', name:'Security Hand', icon:'🛡️', schedule:'0 * * * *', prompt:'Scan for security issues, analyze vulnerabilities, report findings.', phases:'scan,analyze,report', enabled:true, runs:0, tokens:0, cost:0 },
  ];

  const load = async () => {
    try {
      const r = await authFetch('/api/v1/scheduler/tasks');
      const d = await r.json();
      const tasks = d.tasks || [];
      // Map scheduler tasks to hands format, merge with defaults
      if(tasks.length > 0) {
        const mapped = tasks.filter(t => t.name && t.name.includes('Hand')).map(t => ({
          id: t.id, name: t.name, icon: t.icon || '🤚',
          schedule: t.task_type?.Cron?.expression || t.task_type?.Interval ? (t.task_type.Interval.every_secs + 's') : '',
          prompt: t.action?.AgentPrompt?.prompt || '',
          phases: t.phases || '', enabled: t.enabled !== false,
          runs: t.run_count || 0, tokens: t.total_tokens || 0, cost: t.total_cost || 0,
          status: t.status, fail_count: t.fail_count || 0, next_run: t.next_run, last_error: t.last_error
        }));
        if(mapped.length > 0) { setHands(mapped); setLoading(false); return; }
      }
      setHands(defaultHands);
    } catch(e) { setHands(defaultHands); }
    setLoading(false);
  };
  useEffect(() => { load(); }, []);

  const openCreate = () => {
    setEditHand(null);
    setForm({name:'',schedule:'0 */6 * * *',prompt:'',phases:'gather,analyze,report',icon:'🤚'});
    setShowForm(true);
  };
  const openEdit = (h) => {
    setEditHand(h);
    setForm({name:h.name,schedule:h.schedule,prompt:h.prompt||'',phases:h.phases||'',icon:h.icon||'🤚'});
    setShowForm(true);
  };

  const saveHand = async () => {
    if(!form.name.trim()) { showToast('⚠️ Nhập tên Hand','error'); return; }
    try {
      const body = {
        name: form.name,
        task_type: { Cron: { expression: form.schedule } },
        action: { AgentPrompt: { prompt: form.prompt } },
        icon: form.icon,
        phases: form.phases,
        enabled: true,
        retry: { max_retries: 3, delay_secs: 60 }
      };
      if(editHand && editHand.id) {
        const r = await authFetch('/api/v1/scheduler/tasks/'+editHand.id, {
          method:'PUT', headers:{'Content-Type':'application/json'}, body:JSON.stringify(body)
        });
        const d = await r.json();
        if(d.ok || d.id) { showToast('✅ Đã cập nhật: '+form.name,'success'); setShowForm(false); load(); }
        else showToast('❌ '+(d.error||'Lỗi'),'error');
      } else {
        const r = await authFetch('/api/v1/scheduler/tasks', {
          method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify(body)
        });
        const d = await r.json();
        if(d.ok || d.id) { showToast('✅ Đã tạo Hand: '+form.name,'success'); setShowForm(false); load(); }
        else showToast('❌ '+(d.error||'Lỗi'),'error');
      }
    } catch(e) { showToast('❌ '+e.message,'error'); }
  };

  const toggleHand = async (h) => {
    if(h.id && typeof h.id === 'string' && h.id.length > 10) {
      try {
        await authFetch('/api/v1/scheduler/tasks/'+h.id+'/toggle', {
          method:'POST', headers:{'Content-Type':'application/json'},
          body:JSON.stringify({enabled:!h.enabled})
        });
        showToast((h.enabled?'⏸ Đã tắt':'▶ Đã bật')+': '+h.name,'success');
        load();
      } catch(e) { showToast('❌ '+e.message,'error'); }
    } else {
      setHands(prev => prev.map(x => x.id === h.id ? {...x, enabled:!x.enabled} : x));
      showToast((h.enabled?'⏸ Đã tắt':'▶ Đã bật')+': '+h.name,'success');
    }
  };

  const deleteHand = async (h) => {
    if(!confirm('Xoá Hand "'+h.name+'"?')) return;
    if(h.id && typeof h.id === 'string' && h.id.length > 10) {
      try {
        await authFetch('/api/v1/scheduler/tasks/'+h.id, {method:'DELETE'});
        showToast('🗑️ Đã xoá: '+h.name,'success');
        load();
      } catch(e) { showToast('❌ '+e.message,'error'); }
    } else {
      setHands(prev => prev.filter(x => x.id !== h.id));
      showToast('🗑️ Đã xoá: '+h.name,'success');
    }
  };

  const statusBadge = (h) => {
    if(!h.enabled) return html`<span class="badge badge-purple">🚫 disabled</span>`;
    if(h.status === 'Running') return html`<span class="badge badge-yellow">⏳ running</span>`;
    if(h.status === 'Completed') return html`<span class="badge badge-green">✅ done</span>`;
    if(h.status && typeof h.status === 'object' && h.status.Failed) return html`<span class="badge badge-red">❌ failed</span>`;
    if(h.status && typeof h.status === 'object' && h.status.RetryPending) return html`<span class="badge badge-orange">🔄 retry</span>`;
    return html`<span class="badge badge-green">⏹ idle</span>`;
  };

  const activeCount = hands.filter(h => h.enabled).length;
  const totalRuns = hands.reduce((s,h) => s + (h.runs||0), 0);
  const totalCost = hands.reduce((s,h) => s + (h.cost||0), 0);
  const inp = 'width:100%;padding:8px;margin-top:4px;background:var(--bg2);border:1px solid var(--border);border-radius:6px;color:var(--text);font-size:13px';
  const icons = ['🤚','🔍','📊','📝','🔔','🔄','📧','🛡️','🤖','⚡','🌐','💼','🎯','📋','🧹'];

  return html`<div>
    <div class="page-header"><div>
      <h1>🤚 Autonomous Hands</h1>
      <div class="sub">Autonomous agents chạy 24/7 — tạo, cấu hình, quản lý</div>
    </div>
      <button class="btn" style="background:var(--grad1);color:#fff;padding:8px 18px" onClick=${openCreate}>+ Tạo Hand</button>
    </div>
    <div class="stats">
      <${StatsCard} label="Total Hands" value=${hands.length} color="accent" icon="🤚" />
      <${StatsCard} label="Active" value=${activeCount} color="green" icon="▶" />
      <${StatsCard} label="Total Runs" value=${totalRuns} color="blue" icon="🔁" />
      <${StatsCard} label="Total Cost" value=${'$'+totalCost.toFixed(4)} color="orange" icon="💰" />
    </div>

    ${showForm && html`
      <div class="card" style="margin-bottom:14px;border:1px solid var(--accent)">
        <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px">
          <h3>${editHand ? '✏️ Sửa Hand: '+editHand.name : '➕ Tạo Hand mới'}</h3>
          <button class="btn btn-outline btn-sm" onClick=${()=>setShowForm(false)}>✕ Đóng</button>
        </div>
        <div style="display:grid;grid-template-columns:1fr 1fr;gap:10px;font-size:13px">
          <label>Icon
            <div style="display:flex;gap:4px;flex-wrap:wrap;margin-top:4px">
              ${icons.map(ic => html`<button key=${ic} class="btn btn-outline btn-sm" style=${form.icon===ic?'background:var(--accent);color:#fff':''} onClick=${()=>setForm(f=>({...f,icon:ic}))}>${ic}</button>`)}
            </div>
          </label>
          <label>Tên Hand<input style="${inp}" value=${form.name} onInput=${e=>setForm(f=>({...f,name:e.target.value}))} placeholder="My Custom Hand" /></label>
          <label>Schedule (Cron)<input style="${inp}" value=${form.schedule} onInput=${e=>setForm(f=>({...f,schedule:e.target.value}))} placeholder="0 */6 * * * (mỗi 6h)" />
            <div style="font-size:10px;color:var(--text2);margin-top:2px">Cron format: phút giờ ngày tháng thứ. VD: */5 * * * * = mỗi 5 phút</div>
          </label>
          <label>Phases (comma-separated)<input style="${inp}" value=${form.phases} onInput=${e=>setForm(f=>({...f,phases:e.target.value}))} placeholder="gather,analyze,report" /></label>
          <label style="grid-column:span 2">Agent Prompt<textarea style="${inp};min-height:100px;resize:vertical;font-family:var(--mono)" value=${form.prompt} onInput=${e=>setForm(f=>({...f,prompt:e.target.value}))} placeholder="Describe what this hand should do autonomously..." /></label>
        </div>
        <div style="margin-top:12px;display:flex;gap:8px;justify-content:flex-end">
          <button class="btn btn-outline" onClick=${()=>setShowForm(false)}>Huỷ</button>
          <button class="btn" style="background:var(--grad1);color:#fff;padding:8px 20px" onClick=${saveHand}>💾 ${editHand?'Cập nhật':'Tạo'}</button>
        </div>
      </div>
    `}

    <div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(340px,1fr));gap:14px">
      ${hands.map(h => html`<div class="card" key=${h.id} style="border-left:3px solid ${h.enabled?'var(--green)':'var(--text2)'}">
        <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:10px">
          <div style="display:flex;align-items:center;gap:8px">
            <span style="font-size:24px">${h.icon}</span>
            <div><strong>${h.name}</strong><div style="font-size:11px;color:var(--text2)">📅 ${h.schedule}</div></div>
          </div>
          <div style="display:flex;align-items:center;gap:6px">
            ${statusBadge(h)}
            <button class="btn btn-outline btn-sm" onClick=${()=>toggleHand(h)} title=${h.enabled?'Tắt':'Bật'}>${h.enabled?'⏸':'▶'}</button>
            <button class="btn btn-outline btn-sm" onClick=${()=>openEdit(h)} title="Sửa">✏️</button>
            <button class="btn btn-outline btn-sm" style="color:var(--red)" onClick=${()=>deleteHand(h)} title="Xoá">🗑️</button>
          </div>
        </div>
        ${h.prompt && html`<div style="font-size:11px;color:var(--text2);margin-bottom:8px;max-height:40px;overflow:hidden;text-overflow:ellipsis">${h.prompt}</div>`}
        <div style="display:flex;gap:4px;flex-wrap:wrap;margin-bottom:8px">
          ${(h.phases||'').split(',').filter(Boolean).map((p,i) => html`<span key=${i} class="badge badge-blue" style="font-size:10px">${i+1}. ${p.trim()}</span>`)}
        </div>
        <div style="display:grid;grid-template-columns:1fr 1fr 1fr;gap:6px;font-size:11px;color:var(--text2)">
          <div>Runs: <strong style="color:var(--text)">${h.runs||0}</strong></div>
          <div>Tokens: <strong style="color:var(--text)">${h.tokens||0}</strong></div>
          <div>Cost: <strong style="color:var(--orange)">$${(h.cost||0).toFixed(4)}</strong></div>
        </div>
        ${h.last_error && html`<div style="font-size:10px;color:var(--red);margin-top:6px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap" title=${h.last_error}>⚠️ ${h.last_error}</div>`}
      </div>`)}
    </div>
  </div>`;
}

// ═══ SETTINGS PAGE (Tabbed: Provider / Identity / Brain Engine / System Prompt) ═══
function SettingsPage({ config, lang }) {
  const { showToast } = useContext(AppContext);
  const [tab, setTab] = useState('provider');
  const [form, setForm] = useState({provider:'',model:'',agentName:'',persona:'',temperature:0.7,autonomy:'supervised',sysprompt:''});
  const [brainForm, setBrainForm] = useState({enabled:false,mode:'local',model_path:'',threads:4,max_tokens:2048,context_length:4096,temperature:0.7,endpoint:''});
  const [brainHealth, setBrainHealth] = useState(null);
  const [brainFiles, setBrainFiles] = useState([]);
  const [editFile, setEditFile] = useState(null);
  const [fileContent, setFileContent] = useState('');
  const [showNewFile, setShowNewFile] = useState(false);
  const [newFileName, setNewFileName] = useState('');
  const [loading, setLoading] = useState(true);
  const inp = 'width:100%;padding:8px;margin-top:4px;background:var(--bg2);border:1px solid var(--border);border-radius:6px;color:var(--text);font-size:13px';

  useEffect(() => {
    const loadTimeout = setTimeout(() => setLoading(false), 8000); // Safety: never stuck loading > 8s
    (async () => {
      try {
        const r = await authFetch('/api/v1/config');
        const d = await r.json();
        if(d && !d.error) { // Only populate form if API returned valid config
          setForm({
            provider: d.default_provider || '',
            model: d.default_model || '',
            agentName: d.identity?.name || d.agent_name || '',
            persona: d.identity?.persona || d.persona || '',
            temperature: d.default_temperature || d.temperature || 0.7,
            autonomy: d.autonomy?.level || (typeof d.autonomy === 'string' ? d.autonomy : 'supervised'),
            sysprompt: d.identity?.system_prompt || d.system_prompt || ''
          });
          if(d.brain) {
            setBrainForm(f => ({...f,
              enabled: d.brain.enabled || false,
              model_path: d.brain.model_path || '',
              threads: d.brain.threads || 4,
              max_tokens: d.brain.max_tokens || 2048,
              context_length: d.brain.context_length || 4096,
              temperature: d.brain.temperature || 0.7,
            }));
          }
        }
      } catch(e) { console.warn('Settings config load:', e.message); }
      // Load brain health + files (non-critical, fail silently)
      try { const r=await authFetch('/api/v1/health'); setBrainHealth(await r.json()); } catch(e) {}
      try { const r2=await authFetch('/api/v1/brain/files'); const d2=await r2.json(); setBrainFiles(d2.files||[]); } catch(e) {}
      clearTimeout(loadTimeout);
      setLoading(false);
    })();
    return () => clearTimeout(loadTimeout);
  }, []);

  const save = async () => {
    try {
      const body = {
        default_provider: form.provider,
        default_model: form.model,
        identity: { name: form.agentName, persona: form.persona, system_prompt: form.sysprompt },
        default_temperature: form.temperature,
        autonomy: { level: form.autonomy },
        brain: {
          enabled: brainForm.enabled,
          model_path: brainForm.model_path,
          threads: brainForm.threads,
          max_tokens: brainForm.max_tokens,
          context_length: brainForm.context_length,
          temperature: brainForm.temperature,
        }
      };
      const r = await authFetch('/api/v1/config/update', {
        method: 'POST', headers: {'Content-Type':'application/json'},
        body: JSON.stringify(body)
      });
      const d = await r.json();
      if(d.ok) showToast('✅ Đã lưu cấu hình', 'success');
      else showToast('❌ ' + (d.error || 'Lỗi'), 'error');
    } catch(e) { showToast('❌ ' + e.message, 'error'); }
  };

  // Brain file operations
  const openFile = async (name) => {
    try { const r=await authFetch('/api/v1/brain/files/'+encodeURIComponent(name)); const d=await r.json(); setFileContent(d.content||''); setEditFile(name); } catch(e) { showToast('❌ '+e.message,'error'); }
  };
  const saveFile = async () => {
    try { const r=await authFetch('/api/v1/brain/files/'+encodeURIComponent(editFile),{method:'PUT',headers:{'Content-Type':'application/json'},body:JSON.stringify({content:fileContent})}); const d=await r.json(); if(d.ok){showToast('✅ Đã lưu: '+editFile,'success');try{const r2=await authFetch('/api/v1/brain/files');const d2=await r2.json();setBrainFiles(d2.files||[]);}catch(e){}}else showToast('❌ '+(d.error||'Lỗi'),'error');} catch(e){showToast('❌ '+e.message,'error');}
  };
  const createFile = async () => {
    if(!newFileName.trim())return; const fname=newFileName.endsWith('.md')?newFileName:newFileName+'.md';
    try{const r=await authFetch('/api/v1/brain/files/'+encodeURIComponent(fname),{method:'PUT',headers:{'Content-Type':'application/json'},body:JSON.stringify({content:'# '+fname+'\n\n'})});const d=await r.json();if(d.ok){showToast('✅ Đã tạo: '+fname,'success');setShowNewFile(false);setNewFileName('');try{const r2=await authFetch('/api/v1/brain/files');const d2=await r2.json();setBrainFiles(d2.files||[]);}catch(e){}openFile(fname);}else showToast('❌ '+(d.error||'Lỗi'),'error');}catch(e){showToast('❌ '+e.message,'error');}
  };

  if(loading) return html`<div class="card" style="text-align:center;padding:40px;color:var(--text2)">Loading...</div>`;

  const tabs = [
    {id:'provider',icon:'🤖',label:'Nhà cung cấp AI'},
    {id:'identity',icon:'🪪',label:'Danh tính'},
    {id:'brain',icon:'🧠',label:'Brain Engine'},
    {id:'prompt',icon:'📝',label:'System Prompt'},
  ];

  const brainChecks = [
    {name:'SIMD (NEON/AVX)',status:brainHealth?.simd||'—'},{name:'Memory',status:brainHealth?.memory||'—'},
    {name:'Thread Pool',status:brainHealth?.threads||'—'},{name:'GGUF Parser',status:'ready'},
    {name:'KV Cache',status:'initialized'},{name:'Quantization',status:'Q4_K_M, Q5_K_M, Q8_0'},
  ];

  return html`<div>
    <div class="page-header"><div><h1>⚙️ ${t('settings.title',lang)}</h1><div class="sub">${t('settings.subtitle',lang)}</div></div>
      <button class="btn" style="background:var(--grad1);color:#fff;padding:10px 24px" onClick=${save}>💾 ${t('settings.save',lang)}</button>
    </div>

    <div style="display:flex;gap:4px;margin-bottom:16px;border-bottom:1px solid var(--border);padding-bottom:0">
      ${tabs.map(tb => html`<button key=${tb.id}
        class="btn ${tab===tb.id?'':'btn-outline'}" 
        style="padding:8px 16px;border-radius:8px 8px 0 0;font-size:13px;border-bottom:${tab===tb.id?'2px solid var(--accent)':'2px solid transparent'};${tab===tb.id?'background:var(--bg2);color:var(--text)':'color:var(--text2)'}"
        onClick=${()=>setTab(tb.id)}>${tb.icon} ${tb.label}</button>`)}
    </div>

    ${tab==='provider' && html`
      <div style="display:grid;grid-template-columns:1fr 1fr;gap:14px">
        <div class="card"><div class="card-label">🤖 ${t('set.provider_section',lang)}</div>
          <div style="display:grid;gap:10px;font-size:13px">
            <label>${t('set.provider',lang)}<input style="${inp}" value=${form.provider} onInput=${e=>setForm(f=>({...f,provider:e.target.value}))} placeholder="openai, ollama, gemini..." /></label>
            <label>${t('set.model',lang)}<input style="${inp}" value=${form.model} onInput=${e=>setForm(f=>({...f,model:e.target.value}))} placeholder="gpt-4o, llama3..." /></label>
            <label>${t('set.temperature',lang)}: ${form.temperature}<input type="range" min="0" max="2" step="0.1" value=${form.temperature} onInput=${e=>setForm(f=>({...f,temperature:+e.target.value}))} style="width:100%" /></label>
          </div>
        </div>
        <div class="card"><div class="card-label">📋 Thông tin hiện tại</div>
          <div style="display:grid;gap:8px;font-size:13px">
            <div style="display:flex;justify-content:space-between"><span style="color:var(--text2)">Provider:</span><strong>${form.provider||'—'}</strong></div>
            <div style="display:flex;justify-content:space-between"><span style="color:var(--text2)">Model:</span><strong>${form.model||'—'}</strong></div>
            <div style="display:flex;justify-content:space-between"><span style="color:var(--text2)">Temperature:</span><strong>${form.temperature}</strong></div>
            <div style="display:flex;justify-content:space-between"><span style="color:var(--text2)">Autonomy:</span><strong>${form.autonomy}</strong></div>
          </div>
        </div>
      </div>
    `}

    ${tab==='identity' && html`
      <div style="display:grid;grid-template-columns:1fr 1fr;gap:14px">
        <div class="card"><div class="card-label">🪪 ${t('set.identity',lang)}</div>
          <div style="display:grid;gap:10px;font-size:13px">
            <label>${t('set.agent_name',lang)}<input style="${inp}" value=${form.agentName} onInput=${e=>setForm(f=>({...f,agentName:e.target.value}))} /></label>
            <label>${t('set.persona',lang)}<input style="${inp}" value=${form.persona} onInput=${e=>setForm(f=>({...f,persona:e.target.value}))} /></label>
            <label>${t('set.autonomy',lang)}<select style="${inp}" value=${form.autonomy} onChange=${e=>setForm(f=>({...f,autonomy:e.target.value}))}>
              <option value="readonly">${t('set.readonly',lang)}</option><option value="supervised">${t('set.supervised',lang)}</option><option value="full">${t('set.full',lang)}</option>
            </select></label>
          </div>
        </div>
        <div class="card"><div class="card-label">💡 Hướng dẫn</div>
          <div style="font-size:12px;color:var(--text2);line-height:1.8">
            <p><strong>Agent Name:</strong> Tên hiển thị của AI Agent khi trả lời khách hàng.</p>
            <p><strong>Persona:</strong> Vai trò/nhân cách của Agent (ví dụ: "Doanh nhân thân thiện").</p>
            <p><strong>Autonomy:</strong></p>
            <ul style="margin:4px 0;padding-left:16px">
              <li>Readonly: Chỉ trả lời, không thực hiện hành động</li>
              <li>Supervised: Hỏi trước khi hành động</li>
              <li>Full: Tự động thực hiện tất cả</li>
            </ul>
          </div>
        </div>
      </div>
    `}

    ${tab==='brain' && html`
      <div class="card" style="margin-bottom:14px">
        <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:16px">
          <div>
            <div class="card-label" style="margin:0">🧠 Brain Engine — Local LLM</div>
            <div style="font-size:12px;color:var(--text2);margin-top:4px">Chạy AI model trực tiếp trên thiết bị / server của bạn</div>
          </div>
          <label style="display:flex;align-items:center;gap:8px;cursor:pointer">
            <span style="font-size:12px;color:var(--text2)">${brainForm.enabled?'Đang bật':'Đang tắt'}</span>
            <div style="position:relative;width:44px;height:24px;background:${brainForm.enabled?'var(--green)':'var(--bg3)'};border-radius:12px;cursor:pointer;transition:background 0.3s" onClick=${()=>setBrainForm(f=>({...f,enabled:!f.enabled}))}>
              <div style="position:absolute;top:2px;left:${brainForm.enabled?'22px':'2px'};width:20px;height:20px;background:#fff;border-radius:50%;transition:left 0.3s;box-shadow:0 1px 3px rgba(0,0,0,0.3)"></div>
            </div>
          </label>
        </div>

        ${brainForm.enabled && html`
          <div style="display:grid;grid-template-columns:1fr 1fr;gap:14px">
            <div>
              <div style="display:grid;gap:10px;font-size:13px">
                <label>Chế độ<select style="${inp}" value=${brainForm.mode} onChange=${e=>setBrainForm(f=>({...f,mode:e.target.value}))}>
                  <option value="local">🖥️ Local — Chạy trên máy này</option>
                  <option value="shared">🔗 Shared — Dùng chung trên VPS</option>
                  <option value="remote">🌐 Remote — Server LLM riêng</option>
                </select></label>
                ${brainForm.mode==='remote' && html`<label>Endpoint URL<input style="${inp}" value=${brainForm.endpoint} onInput=${e=>setBrainForm(f=>({...f,endpoint:e.target.value}))} placeholder="http://gpu-server:8080" /></label>`}
                <label>Model Path (GGUF)<input style="${inp}" value=${brainForm.model_path} onInput=${e=>setBrainForm(f=>({...f,model_path:e.target.value}))} placeholder="/models/qwen2-7b-q4.gguf" /></label>
                <label>Threads<input type="number" style="${inp}" value=${brainForm.threads} onInput=${e=>setBrainForm(f=>({...f,threads:+e.target.value||4}))} min="1" max="32" /></label>
                <label>Max Tokens<input type="number" style="${inp}" value=${brainForm.max_tokens} onInput=${e=>setBrainForm(f=>({...f,max_tokens:+e.target.value||2048}))} /></label>
                <label>Context Length<input type="number" style="${inp}" value=${brainForm.context_length} onInput=${e=>setBrainForm(f=>({...f,context_length:+e.target.value||4096}))} /></label>
                <label>Temperature: ${brainForm.temperature}<input type="range" min="0" max="2" step="0.1" value=${brainForm.temperature} onInput=${e=>setBrainForm(f=>({...f,temperature:+e.target.value}))} style="width:100%" /></label>
              </div>
            </div>
            <div>
              <h4 style="margin-bottom:8px;font-size:13px">🏥 Health Checks</h4>
              <div style="display:grid;gap:4px">
                ${brainChecks.map(c=>html`<div key=${c.name} style="display:flex;align-items:center;gap:8px;padding:6px 10px;background:var(--bg2);border-radius:6px;font-size:12px">
                  <span>✅</span><strong style="flex:1">${c.name}</strong><span style="color:var(--text2)">${c.status}</span>
                </div>`)}
              </div>
            </div>
          </div>
        `}
      </div>

      <div class="card">
        <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px">
          <h3 style="margin:0">📁 Brain Workspace</h3>
          <button class="btn" style="background:var(--grad1);color:#fff;padding:6px 14px;font-size:12px" onClick=${()=>setShowNewFile(!showNewFile)}>+ Tạo file</button>
        </div>
        ${showNewFile && html`<div style="display:flex;gap:8px;margin-bottom:10px"><input style="${inp};flex:1" value=${newFileName} onInput=${e=>setNewFileName(e.target.value)} placeholder="MY_FILE.md" /><button class="btn" style="background:var(--grad1);color:#fff;padding:6px 14px" onClick=${createFile}>Tạo</button></div>`}
        ${editFile && html`<div style="margin-bottom:10px;border:1px solid var(--accent);border-radius:8px;padding:10px">
          <div style="display:flex;justify-content:space-between;margin-bottom:6px"><strong>📝 ${editFile}</strong><div style="display:flex;gap:4px"><button class="btn" style="background:var(--grad1);color:#fff;padding:4px 12px;font-size:12px" onClick=${saveFile}>💾 Lưu</button><button class="btn btn-outline btn-sm" onClick=${()=>setEditFile(null)}>✕</button></div></div>
          <textarea value=${fileContent} onInput=${e=>setFileContent(e.target.value)} style="width:100%;min-height:200px;padding:10px;background:var(--bg2);border:1px solid var(--border);border-radius:6px;color:var(--text);font-family:var(--mono);font-size:12px;resize:vertical" />
        </div>`}
        ${brainFiles.length===0 ? html`<div style="text-align:center;padding:20px;color:var(--text2);font-size:13px">Workspace trống. Click "+ Tạo file" để bắt đầu.</div>` : html`<div style="display:grid;gap:4px">
          ${brainFiles.map(f=>html`<div key=${f.name||f} style="display:flex;align-items:center;gap:8px;padding:6px 10px;background:var(--bg2);border-radius:4px;font-size:13px;cursor:pointer" onClick=${()=>openFile(f.name||f)}>
            <span>📄</span><span style="flex:1">${f.name||f}</span><span style="color:var(--text2);font-size:11px">${f.size||''}</span><span class="badge badge-blue" style="font-size:10px">✏️ Edit</span>
          </div>`)}
        </div>`}
      </div>
    `}

    ${tab==='prompt' && html`
      <div class="card"><div class="card-label">📝 System Prompt</div>
        <div style="font-size:12px;color:var(--text2);margin-bottom:10px">Hướng dẫn chung cho AI Agent — prompt này sẽ được gửi trước mỗi cuộc hội thoại.</div>
        <textarea style="width:100%;min-height:250px;padding:12px;background:var(--bg2);border:1px solid var(--border);border-radius:8px;color:var(--text);font-family:var(--mono);font-size:13px;resize:vertical;line-height:1.6" value=${form.sysprompt} onInput=${e=>setForm(f=>({...f,sysprompt:e.target.value}))} placeholder="You are a helpful AI assistant..." />
      </div>
    `}
  </div>`;
}

// ═══ PROVIDERS PAGE (with Configure + Activate) ═══
function ProvidersPage({ config, lang }) {
  const { showToast } = useContext(AppContext);
  const [providers, setProviders] = useState([]);
  const [loading, setLoading] = useState(true);
  const [configProv, setConfigProv] = useState(null);
  const [provForm, setProvForm] = useState({api_key:'',base_url:'',model:''});

  const load = async () => {
    try { const r=await authFetch('/api/v1/providers'); const d=await r.json(); setProviders(d.providers||[]); } catch(e){}
    setLoading(false);
  };
  useEffect(()=>{ load(); },[]);

  const active = config?.default_provider || '';
  const typeColor = t => t==='cloud'?'badge-blue':t==='local'?'badge-green':'badge-purple';

  const openConfig = (p) => {
    setConfigProv(p);
    setProvForm({api_key:p.api_key||'',base_url:p.base_url||'',model:(p.models||[])[0]||''});
  };

  const activateProvider = async (name, model) => {
    try {
      const r = await authFetch('/api/v1/config/update', {
        method:'POST', headers:{'Content-Type':'application/json'},
        body:JSON.stringify({default_provider:name, default_model:model||''})
      });
      const d=await r.json();
      if(d.ok) showToast('⚡ Activated: '+name,'success');
      else showToast('❌ '+(d.error||'Lỗi'),'error');
    } catch(e) { showToast('❌ '+e.message,'error'); }
  };

  const saveProviderConfig = async () => {
    try {
      const body = { api_key: provForm.api_key, base_url: provForm.base_url };
      const r = await authFetch('/api/v1/providers/' + encodeURIComponent(configProv.name), {
        method:'PUT', headers:{'Content-Type':'application/json'}, body:JSON.stringify(body)
      });
      const d=await r.json();
      if(d.ok) { showToast('✅ Đã cấu hình: '+configProv.name,'success'); setConfigProv(null); load(); }
      else showToast('❌ '+(d.error||'Lỗi'),'error');
    } catch(e) { showToast('❌ '+e.message,'error'); }
  };

  const inp = 'width:100%;padding:8px;margin-top:4px;background:var(--bg2);border:1px solid var(--border);border-radius:6px;color:var(--text);font-size:13px';

  return html`<div>
    <div class="page-header"><div><h1>🔌 ${t('providers.title',lang)}</h1><div class="sub">${t('providers.subtitle',lang)}</div></div></div>
    <div class="stats">
      <${StatsCard} label=${t('providers.active_label',lang)} value=${active||'—'} color="green" icon="⚡" />
      <${StatsCard} label=${t('providers.total_label',lang)} value=${providers.length} color="accent" icon="🔌" />
    </div>

    ${configProv && html`
      <div class="card" style="margin-bottom:14px;border:1px solid var(--accent)">
        <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px">
          <h3>🔑 Cấu hình ${configProv.label||configProv.name}</h3>
          <button class="btn btn-outline btn-sm" onClick=${()=>setConfigProv(null)}>✕ Đóng</button>
        </div>
        <div style="display:grid;grid-template-columns:1fr 1fr;gap:10px;font-size:13px">
          <label>API Key<input style="${inp}" type="password" value=${provForm.api_key} onInput=${e=>setProvForm(f=>({...f,api_key:e.target.value}))} placeholder="sk-..." /></label>
          <label>Base URL<input style="${inp}" value=${provForm.base_url} onInput=${e=>setProvForm(f=>({...f,base_url:e.target.value}))} placeholder="https://api.openai.com/v1" /></label>
          <label style="grid-column:span 2">Default Model<input style="${inp}" value=${provForm.model} onInput=${e=>setProvForm(f=>({...f,model:e.target.value}))} placeholder="gpt-4o, llama3.2..." /></label>
        </div>
        <div style="margin-top:12px;display:flex;gap:8px;justify-content:flex-end">
          <button class="btn btn-outline" onClick=${()=>setConfigProv(null)}>Huỷ</button>
          <button class="btn" style="background:var(--grad1);color:#fff;padding:8px 20px" onClick=${saveProviderConfig}>💾 Lưu</button>
        </div>
      </div>
    `}

    <div class="card">${loading?html`<div style="text-align:center;padding:20px;color:var(--text2)">Loading...</div>`:html`<table><thead><tr><th></th><th>Provider</th><th>Type</th><th>Models</th><th>Status</th><th style="text-align:right">Thao tác</th></tr></thead><tbody>
      ${providers.map(p=>html`<tr key=${p.name}><td style="font-size:20px">${p.icon||'🤖'}</td><td><strong>${p.label||p.name}</strong></td><td><span class="badge ${typeColor(p.provider_type)}">${p.provider_type}</span></td><td style="font-size:12px">${(p.models||[]).slice(0,3).join(', ')}</td><td>${p.name===active?html`<span class="badge badge-green">✅ Active</span>`:html`<span class="badge">—</span>`}</td>
        <td style="text-align:right;white-space:nowrap">
          <button class="btn btn-outline btn-sm" onClick=${()=>openConfig(p)} title="Cấu hình">🔑</button>
          ${p.name!==active?html`<button class="btn btn-outline btn-sm" style="margin-left:4px" onClick=${()=>activateProvider(p.name,(p.models||[])[0])} title="Kích hoạt">⚡</button>`:''}
        </td>
      </tr>`)}
    </tbody></table>`}</div>
  </div>`;
}

// ═══ CHANNELS PAGE — Full rewrite with proper per-channel config ═══
function ChannelsPage({ lang }) {
  const { showToast } = useContext(AppContext);
  const [channelData, setChannelData] = useState(null);
  const [apiChannels, setApiChannels] = useState([]);
  const [loading, setLoading] = useState(true);
  const [configCh, setConfigCh] = useState(null);
  const [chForm, setChForm] = useState({});
  const [zaloQr, setZaloQr] = useState(null);
  const [zaloLoading, setZaloLoading] = useState(false);
  const inp = 'width:100%;padding:8px;margin-top:4px;background:var(--bg2);border:1px solid var(--border);border-radius:6px;color:var(--text);font-size:13px';

  // Channel definitions with proper field mappings
  const channelDefs = [
    {name:'cli',icon:'💻',label:'CLI Terminal',type:'interactive',alwaysActive:true},
    {name:'telegram',icon:'📱',label:'Telegram Bot',type:'messaging',
     fields:[{key:'bot_token',label:'Bot Token',secret:true},{key:'allowed_chat_ids',label:'Allowed Chat IDs',placeholder:'-100123, 987654'}]},
    {name:'zalo',icon:'💙',label:'Zalo Personal',type:'messaging',hasQr:true,
     fields:[{key:'cookie',label:'Cookie (từ chat.zalo.me)',secret:true,textarea:true},{key:'imei',label:'IMEI (Device ID)',placeholder:'Tự tạo nếu để trống'}]},
    {name:'discord',icon:'🎮',label:'Discord Bot',type:'messaging',
     fields:[{key:'bot_token',label:'Bot Token',secret:true},{key:'allowed_channel_ids',label:'Allowed Channel IDs',placeholder:'123456, 789012'}]},
    {name:'email',icon:'📧',label:'Email (IMAP/SMTP)',type:'messaging',
     fields:[{key:'smtp_host',label:'SMTP Host',placeholder:'smtp.gmail.com'},{key:'smtp_port',label:'SMTP Port',placeholder:'587'},
             {key:'smtp_user',label:'Email Address',placeholder:'bot@example.com'},{key:'smtp_pass',label:'App Password',secret:true},
             {key:'imap_host',label:'IMAP Host',placeholder:'imap.gmail.com'}]},
    {name:'whatsapp',icon:'💬',label:'WhatsApp Business',type:'messaging',
     fields:[{key:'phone_number_id',label:'Phone Number ID'},{key:'access_token',label:'Access Token',secret:true},{key:'business_id',label:'Business ID'}]},
    {name:'webhook',icon:'🌐',label:'Webhook',type:'api',
     fields:[{key:'webhook_url',label:'Outbound URL',placeholder:'https://example.com/webhook'},{key:'webhook_secret',label:'Secret',secret:true}]},
  ];

  const load = async () => {
    try {
      // Load both config and channel list
      const [cfgRes, chRes] = await Promise.all([
        authFetch('/api/v1/config'),
        authFetch('/api/v1/channels'),
      ]);
      const cfgData = await cfgRes.json();
      const chData = await chRes.json();
      setChannelData(cfgData.channels || {});
      setApiChannels(chData.channels || []);
    } catch(e) {
      console.error('Channels load:', e);
      setChannelData({});
      setApiChannels([]);
    }
    setLoading(false);
  };
  useEffect(() => {
    const t = setTimeout(() => setLoading(false), 8000);
    load().finally(() => clearTimeout(t));
    return () => clearTimeout(t);
  }, []);

  const getStatus = (name) => {
    if(name === 'cli') return 'active';
    const apiCh = apiChannels.find(c => c.name === name);
    if(apiCh?.status === 'active') return 'active';
    const cfgCh = channelData?.[name];
    if(cfgCh?.enabled) return 'active';
    if(cfgCh) return 'configured';
    return 'available';
  };

  const openConfig = (def) => {
    setConfigCh(def);
    setZaloQr(null);
    // Pre-fill form from config data
    const cfgCh = channelData?.[def.name] || {};
    const f = { enabled: getStatus(def.name) === 'active' };
    (def.fields || []).forEach(fd => {
      f[fd.key] = cfgCh[fd.key] || '';
    });
    setChForm(f);
  };

  const saveChannelConfig = async () => {
    if(!configCh) return;
    try {
      const body = { channel_type: configCh.name, enabled: chForm.enabled !== false, ...chForm };
      const r = await authFetch('/api/v1/channels/update', {
        method: 'POST', headers: {'Content-Type':'application/json'},
        body: JSON.stringify(body)
      });
      const d = await r.json();
      if(d.ok) { showToast('✅ Đã cấu hình '+configCh.label,'success'); setConfigCh(null); load(); }
      else showToast('❌ '+(d.error||d.message||'Lỗi'),'error');
    } catch(e) { showToast('❌ '+e.message,'error'); }
  };

  const loadZaloQr = async () => {
    setZaloLoading(true);
    try {
      const r = await authFetch('/api/v1/zalo/qr', { method: 'POST' });
      const d = await r.json();
      if(d.ok) { setZaloQr(d); if(d.imei) setChForm(f=>({...f,imei:d.imei})); }
      else showToast('❌ '+(d.error||'Không thể tạo QR'),'error');
    } catch(e) { showToast('❌ '+e.message,'error'); }
    setZaloLoading(false);
  };

  const statusBadge = s => {
    if(s==='active') return html`<span class="badge badge-green" style="font-size:11px">● Hoạt động</span>`;
    if(s==='configured') return html`<span class="badge badge-blue" style="font-size:11px">✓ Đã cấu hình</span>`;
    return html`<span class="badge" style="font-size:11px">○ Chưa kết nối</span>`;
  };

  if(loading) return html`<div class="card" style="text-align:center;padding:40px;color:var(--text2)">Đang tải kênh liên lạc...</div>`;

  const activeCount = channelDefs.filter(d => getStatus(d.name)==='active').length;
  const configuredCount = channelDefs.filter(d => getStatus(d.name)==='configured').length;

  return html`<div>
    <div class="page-header"><div><h1>📱 ${t('channels.title',lang)}</h1><div class="sub">${t('channels.subtitle',lang)} — ${channelDefs.length} kênh liên lạc</div></div></div>
    <div class="stats">
      <${StatsCard} label="Tổng kênh" value=${channelDefs.length} color="accent" icon="📱" />
      <${StatsCard} label="Hoạt động" value=${activeCount} color="green" icon="✅" />
      <${StatsCard} label="Đã cấu hình" value=${configuredCount} color="blue" icon="🔧" />
    </div>

    ${configCh && html`
      <div class="card" style="margin-bottom:14px;border:1px solid var(--accent)">
        <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:14px">
          <h3 style="margin:0">${configCh.icon} Cấu hình ${configCh.label}</h3>
          <button class="btn btn-outline btn-sm" onClick=${()=>setConfigCh(null)}>✕ Đóng</button>
        </div>

        <div style="display:flex;align-items:center;gap:8px;margin-bottom:14px;padding:10px;background:var(--bg2);border-radius:8px">
          <span style="font-size:13px">Kích hoạt kênh:</span>
          <div style="position:relative;width:44px;height:24px;background:${chForm.enabled?'var(--green)':'var(--bg3)'};border-radius:12px;cursor:pointer;transition:background 0.3s" onClick=${()=>setChForm(f=>({...f,enabled:!f.enabled}))}>
            <div style="position:absolute;top:2px;left:${chForm.enabled?'22px':'2px'};width:20px;height:20px;background:#fff;border-radius:50%;transition:left 0.3s;box-shadow:0 1px 3px rgba(0,0,0,0.3)"></div>
          </div>
          <span style="font-size:12px;color:${chForm.enabled?'var(--green)':'var(--text2)'}">${chForm.enabled?'Đang bật':'Đang tắt'}</span>
        </div>

        ${configCh.hasQr && html`
          <div style="margin-bottom:14px;padding:12px;background:var(--bg2);border-radius:8px;border:1px solid var(--border)">
            <div style="display:flex;align-items:center;gap:8px;margin-bottom:8px">
              <strong style="font-size:13px">📱 Đăng nhập Zalo bằng QR</strong>
              <button class="btn" style="background:var(--grad1);color:#fff;padding:4px 12px;font-size:12px" onClick=${loadZaloQr} disabled=${zaloLoading}>${zaloLoading?'Đang tạo...':'🔲 Quét QR'}</button>
            </div>
            ${zaloQr && html`
              <div style="text-align:center;padding:10px">
                ${zaloQr.qr_code ? html`<img src="data:image/png;base64,${zaloQr.qr_code}" style="width:200px;height:200px;border-radius:8px;border:2px solid var(--accent)" />` : html`<div style="color:var(--text2)">Không thể hiển thị QR</div>`}
                <div style="font-size:12px;color:var(--text2);margin-top:8px">${zaloQr.message || 'Quét mã QR bằng Zalo trên điện thoại'}</div>
              </div>
            `}
            <div style="font-size:11px;color:var(--text2);margin-top:6px">Hoặc paste cookie từ chat.zalo.me vào ô bên dưới</div>
          </div>
        `}

        <div style="display:grid;gap:10px;font-size:13px">
          ${(configCh.fields||[]).map(fd => html`
            <label key=${fd.key}>${fd.label}
              ${fd.textarea ? html`<textarea style="${inp};min-height:80px;font-family:var(--mono);resize:vertical" value=${chForm[fd.key]||''} onInput=${e=>setChForm(f=>({...f,[fd.key]:e.target.value}))} placeholder=${fd.placeholder||'Nhập '+fd.label+'...'} />` :
              html`<input style="${inp}" type=${fd.secret?'password':'text'} value=${chForm[fd.key]||''} onInput=${e=>setChForm(f=>({...f,[fd.key]:e.target.value}))} placeholder=${fd.placeholder||'Nhập '+fd.label+'...'} />`}
            </label>
          `)}
        </div>
        <div style="margin-top:14px;display:flex;gap:8px;justify-content:flex-end">
          <button class="btn btn-outline" onClick=${()=>setConfigCh(null)}>Huỷ</button>
          <button class="btn" style="background:var(--grad1);color:#fff;padding:8px 20px" onClick=${saveChannelConfig}>💾 Lưu cấu hình</button>
        </div>
      </div>
    `}

    <div class="card"><div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(280px,1fr));gap:10px">
      ${channelDefs.map(def => {
        const st = getStatus(def.name);
        return html`<div key=${def.name} style="display:flex;align-items:center;gap:10px;padding:12px 14px;background:var(--bg2);border-radius:8px;border:1px solid ${st==='active'?'var(--green)':st==='configured'?'var(--accent)':'var(--border)'}">
          <span style="font-size:24px">${def.icon}</span>
          <div style="flex:1">
            <strong style="font-size:13px">${def.label}</strong>
            <div style="font-size:11px;color:var(--text2)">${def.type}</div>
          </div>
          ${statusBadge(st)}
          ${def.fields && html`<button class="btn btn-outline btn-sm" onClick=${()=>openConfig(def)} title="Cấu hình">⚙️</button>`}
        </div>`;
      })}
    </div></div>
  </div>`;
}

// ═══ TOOLS PAGE (with Enable/Disable) ═══
function ToolsPage({ lang }) {
  const { showToast } = useContext(AppContext);
  const defaultTools = [
    {name:'shell',icon:'🖥️',desc:t('tool.shell_desc',lang),enabled:true},{name:'file',icon:'📁',desc:t('tool.file_desc',lang),enabled:true},
    {name:'edit_file',icon:'✏️',desc:t('tool.editfile_desc',lang),enabled:true},{name:'glob',icon:'🔍',desc:t('tool.glob_desc',lang),enabled:true},
    {name:'grep',icon:'🔎',desc:t('tool.grep_desc',lang),enabled:true},{name:'http_request',icon:'🌐',desc:t('tool.httpreq_desc',lang),enabled:true},
    {name:'execute_code',icon:'⚡',desc:t('tool.execcode_desc',lang),enabled:true},{name:'web_search',icon:'🔍',desc:'DuckDuckGo, SearXNG',enabled:true},
    {name:'plan',icon:'📋',desc:t('tool.plan_desc',lang),enabled:true},{name:'session_context',icon:'📊',desc:t('tool.sessionctx_desc',lang),enabled:true},
    {name:'config_manager',icon:'⚙️',desc:t('tool.configmgr_desc',lang),enabled:true},{name:'memory_search',icon:'🧠',desc:t('tool.memsearch_desc',lang),enabled:true},
    {name:'doc_reader',icon:'📄',desc:t('tool.docreader_desc',lang),enabled:true},
  ];
  const [tools, setTools] = useState(defaultTools);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    (async () => {
      try { const r=await authFetch('/api/v1/tools'); const d=await r.json(); if(d.tools && d.tools.length) setTools(d.tools); else setTools(defaultTools); }
      catch(e) { setTools(defaultTools); }
      setLoading(false);
    })();
  }, []);

  const toggleTool = async (name) => {
    const updated = tools.map(t => t.name===name ? {...t,enabled:!t.enabled} : t);
    setTools(updated);
    try {
      await authFetch('/api/v1/tools/'+name+'/toggle', {
        method:'POST', headers:{'Content-Type':'application/json'},
        body:JSON.stringify({enabled:!tools.find(t=>t.name===name).enabled})
      });
      showToast((updated.find(t=>t.name===name).enabled?'✅ Bật':'⏸ Tắt')+': '+name,'success');
    } catch(e) { /* API may not exist yet, local toggle is fine */ }
  };

  const active = tools.filter(t=>t.enabled).length;

  return html`<div>
    <div class="page-header"><div><h1>🛠️ ${t('tools.title',lang)}</h1><div class="sub">${t('tools.subtitle',lang)}</div></div></div>
    <div class="stats"><${StatsCard} label="Native Tools" value=${tools.length} color="accent" icon="🛠️" /><${StatsCard} label="Enabled" value=${active} color="green" icon="✅" /><${StatsCard} label="MCP Tools" value="∞" color="blue" icon="🔗" /></div>
    <div class="card"><div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(280px,1fr));gap:10px">
      ${tools.map(tl=>html`<div key=${tl.name} style="display:flex;align-items:flex-start;gap:10px;padding:12px;background:var(--bg2);border-radius:8px;border:1px solid var(--border);opacity:${tl.enabled?1:0.5}">
        <span style="font-size:24px">${tl.icon}</span>
        <div style="flex:1"><strong style="font-size:13px">${tl.name}</strong><div style="font-size:11px;color:var(--text2);margin-top:2px">${tl.desc}</div></div>
        <button class="btn btn-outline btn-sm" onClick=${()=>toggleTool(tl.name)} title=${tl.enabled?'Tắt':'Bật'}>${tl.enabled?'✅':'⏸'}</button>
      </div>`)}
    </div></div>
  </div>`;
}

// ═══ MCP SERVERS PAGE (with Add/Remove) ═══
function McpPage({ lang }) {
  const { showToast } = useContext(AppContext);
  const [servers,setServers] = useState([]);
  const [loading,setLoading] = useState(true);
  const [showAdd,setShowAdd] = useState(false);
  const [addForm,setAddForm] = useState({name:'',command:'npx',args:'',env:''});

  const popular = [
    {name:'filesystem',desc:'Read/write filesystem',icon:'📁',cmd:'npx -y @modelcontextprotocol/server-filesystem /tmp'},
    {name:'github',desc:'GitHub API',icon:'🐙',cmd:'npx -y @modelcontextprotocol/server-github'},
    {name:'postgres',desc:'PostgreSQL queries',icon:'🐘',cmd:'npx -y @modelcontextprotocol/server-postgres'},
    {name:'slack',desc:'Slack integration',icon:'💬',cmd:'npx -y @modelcontextprotocol/server-slack'},
    {name:'puppeteer',desc:'Browser automation',icon:'🌐',cmd:'npx -y @modelcontextprotocol/server-puppeteer'},
    {name:'memory',desc:'Knowledge graph',icon:'🧠',cmd:'npx -y @modelcontextprotocol/server-memory'},
    {name:'gdrive',desc:'Google Drive',icon:'📂',cmd:'npx -y @anthropic/server-gdrive'},
    {name:'sqlite',desc:'SQLite database',icon:'💾',cmd:'npx -y @modelcontextprotocol/server-sqlite'},
  ];

  const load = async () => {
    try{const r=await authFetch('/api/v1/mcp/servers');const d=await r.json();setServers(d.servers||[]);}catch(e){}
    setLoading(false);
  };
  useEffect(()=>{ load(); },[]);

  const addServer = async () => {
    if(!addForm.name.trim()) { showToast('⚠️ Nhập tên','error'); return; }
    try {
      const args = addForm.args ? addForm.args.split(' ') : [];
      const body = { name:addForm.name, command:addForm.command, args, env:addForm.env?JSON.parse(addForm.env):{} };
      const r = await authFetch('/api/v1/mcp/servers', {
        method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify(body)
      });
      const d=await r.json();
      if(d.ok) { showToast('✅ Đã thêm: '+addForm.name,'success'); setShowAdd(false); setAddForm({name:'',command:'npx',args:'',env:''}); load(); }
      else showToast('❌ '+(d.error||'Lỗi'),'error');
    } catch(e) { showToast('❌ '+e.message,'error'); }
  };

  const removeServer = async (name) => {
    if(!confirm('Xoá MCP server "'+name+'"?')) return;
    try {
      const r = await authFetch('/api/v1/mcp/servers/'+encodeURIComponent(name), {method:'DELETE'});
      const d=await r.json();
      if(d.ok) { showToast('🗑️ Đã xoá: '+name,'success'); load(); }
      else showToast('❌ '+(d.error||'Lỗi'),'error');
    } catch(e) { showToast('❌ '+e.message,'error'); }
  };

  const quickAdd = (p) => {
    const parts = p.cmd.split(' ');
    setAddForm({name:p.name, command:parts[0], args:parts.slice(1).join(' '), env:''});
    setShowAdd(true);
  };

  const inp = 'width:100%;padding:8px;margin-top:4px;background:var(--bg2);border:1px solid var(--border);border-radius:6px;color:var(--text);font-size:13px';

  return html`<div>
    <div class="page-header"><div><h1>🔗 ${t('mcp.title',lang)}</h1><div class="sub">${t('mcp.subtitle',lang)}</div></div>
      <button class="btn" style="background:var(--grad1);color:#fff;padding:8px 18px" onClick=${()=>setShowAdd(!showAdd)}>+ Thêm Server</button>
    </div>
    <div class="stats">
      <${StatsCard} label=${t('mcp.total',lang)} value=${servers.length} color="accent" icon="🔗" />
      <${StatsCard} label=${t('mcp.active',lang)} value=${servers.filter(s=>s.status==='connected').length} color="green" icon="✅" />
    </div>

    ${showAdd && html`
      <div class="card" style="margin-bottom:14px;border:1px solid var(--accent)">
        <h3 style="margin-bottom:10px">🔌 Thêm MCP Server</h3>
        <div style="display:grid;grid-template-columns:1fr 1fr;gap:10px;font-size:13px">
          <label>Tên server<input style="${inp}" value=${addForm.name} onInput=${e=>setAddForm(f=>({...f,name:e.target.value}))} placeholder="filesystem" /></label>
          <label>Command<input style="${inp}" value=${addForm.command} onInput=${e=>setAddForm(f=>({...f,command:e.target.value}))} placeholder="npx" /></label>
          <label style="grid-column:span 2">Arguments<input style="${inp}" value=${addForm.args} onInput=${e=>setAddForm(f=>({...f,args:e.target.value}))} placeholder="-y @modelcontextprotocol/server-filesystem /tmp" /></label>
        </div>
        <div style="margin-top:12px;display:flex;gap:8px;justify-content:flex-end">
          <button class="btn btn-outline" onClick=${()=>setShowAdd(false)}>Huỷ</button>
          <button class="btn" style="background:var(--grad1);color:#fff;padding:8px 20px" onClick=${addServer}>💾 Thêm</button>
        </div>
      </div>
    `}

    <div class="card"><h3 style="margin-bottom:12px">🔌 ${t('mcp.popular',lang)} — Quick Add</h3>
      <div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(220px,1fr));gap:10px">
        ${popular.map(p=>html`<div key=${p.name} style="display:flex;align-items:center;gap:10px;padding:10px 14px;background:var(--bg2);border-radius:8px;border:1px solid var(--border)">
          <span style="font-size:22px">${p.icon}</span>
          <div style="flex:1"><strong style="font-size:13px">${p.name}</strong><div style="font-size:11px;color:var(--text2)">${p.desc}</div></div>
          <button class="btn btn-outline btn-sm" onClick=${()=>quickAdd(p)} title="Quick Add">+</button>
        </div>`)}
      </div>
    </div>
    ${servers.length>0&&html`<div class="card" style="margin-top:14px"><h3 style="margin-bottom:12px">📡 Connected Servers (${servers.length})</h3>
      <table><thead><tr><th>Server</th><th>Protocol</th><th>Tools</th><th>Status</th><th style="text-align:right">Thao tác</th></tr></thead><tbody>
        ${servers.map(s=>html`<tr key=${s.name}><td><strong>${s.name}</strong></td><td><span class="badge badge-blue">${s.transport||'stdio'}</span></td><td>${s.tools_count||0}</td><td><span class="badge ${s.status==='connected'?'badge-green':'badge-red'}">${s.status}</span></td>
          <td style="text-align:right"><button class="btn btn-outline btn-sm" style="color:var(--red)" onClick=${()=>removeServer(s.name)} title="Xoá">🗑️</button></td>
        </tr>`)}
      </tbody></table>
    </div>`}
  </div>`;
}
// ═══ AGENTS PAGE (Full CRUD) ═══
function AgentsPage({ config, lang }) {
  const { showToast } = useContext(AppContext);
  const [agents,setAgents] = useState([]);
  const [loading,setLoading] = useState(true);
  const [showForm,setShowForm] = useState(false);
  const [editAgent,setEditAgent] = useState(null);
  const [form,setForm] = useState({name:'',role:'',description:'',system_prompt:'',provider:'',model:''});

  const load = async () => {
    try { const r=await authFetch('/api/v1/agents');const d=await r.json();setAgents(d.agents||[]); } catch(e){ console.error('AgentsPage load error:', e); }
    setLoading(false);
  };
  useEffect(()=>{ load(); },[]);

  const openCreate = () => { setEditAgent(null); setForm({name:'',role:'general',description:'',system_prompt:'',provider:config?.default_provider||'',model:config?.default_model||''}); setShowForm(true); };
  const openEdit = (a) => { setEditAgent(a); setForm({name:a.name,role:a.role||'',description:a.description||'',system_prompt:a.system_prompt||'',provider:a.provider||'',model:a.model||''}); setShowForm(true); };

  const saveAgent = async () => {
    try {
      if(editAgent) {
        const r = await authFetch('/api/v1/agents/'+encodeURIComponent(editAgent.name), {
          method:'PUT', headers:{'Content-Type':'application/json'}, body:JSON.stringify(form)
        });
        const d=await r.json();
        if(d.ok) { showToast('✅ Đã cập nhật agent: '+form.name,'success'); load(); setShowForm(false); }
        else showToast('❌ '+(d.error||'Lỗi'),'error');
      } else {
        const r = await authFetch('/api/v1/agents', {
          method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify(form)
        });
        const d=await r.json();
        if(d.ok) { showToast('✅ Đã tạo agent: '+form.name,'success'); load(); setShowForm(false); }
        else showToast('❌ '+(d.error||'Lỗi'),'error');
      }
    } catch(e) { showToast('❌ '+e.message,'error'); }
  };

  const deleteAgent = async (name) => {
    if(!confirm('Xoá agent "'+name+'"?')) return;
    try {
      const r = await authFetch('/api/v1/agents/'+encodeURIComponent(name), {method:'DELETE'});
      const d=await r.json();
      if(d.ok) { showToast('🗑️ Đã xoá: '+name,'success'); load(); }
      else showToast('❌ '+(d.error||'Lỗi'),'error');
    } catch(e) { showToast('❌ '+e.message,'error'); }
  };

  const inp = 'width:100%;padding:8px;margin-top:4px;background:var(--bg2);border:1px solid var(--border);border-radius:6px;color:var(--text);font-size:13px';

  return html`<div>
    <div class="page-header"><div><h1>🤖 ${t('agents.title',lang)}</h1><div class="sub">${t('agents.subtitle',lang)}</div></div>
      <button class="btn" style="background:var(--grad1);color:#fff;padding:8px 18px" onClick=${openCreate}>+ Tạo Agent</button>
    </div>
    <div class="stats"><${StatsCard} label=${t('agents.total',lang)} value=${agents.length} color="accent" icon="🤖" /></div>

    ${showForm && html`
      <div class="card" style="margin-bottom:14px;border:1px solid var(--accent)">
        <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px">
          <h3>${editAgent ? '✏️ Sửa Agent: '+editAgent.name : '➕ Tạo Agent mới'}</h3>
          <button class="btn btn-outline btn-sm" onClick=${()=>setShowForm(false)}>✕ Đóng</button>
        </div>
        <div style="display:grid;grid-template-columns:1fr 1fr;gap:10px;font-size:13px">
          <label>Tên Agent<input style="${inp}" value=${form.name} onInput=${e=>setForm(f=>({...f,name:e.target.value}))} placeholder="sales-bot" ${editAgent?'disabled':''} /></label>
          <label>Vai trò<input style="${inp}" value=${form.role} onInput=${e=>setForm(f=>({...f,role:e.target.value}))} placeholder="coder, writer, analyst..." /></label>
          <label>Provider<input style="${inp}" value=${form.provider} onInput=${e=>setForm(f=>({...f,provider:e.target.value}))} placeholder="ollama, openai..." /></label>
          <label>Model<input style="${inp}" value=${form.model} onInput=${e=>setForm(f=>({...f,model:e.target.value}))} placeholder="llama3.2, gpt-4o..." /></label>
          <label style="grid-column:span 2">Mô tả<input style="${inp}" value=${form.description} onInput=${e=>setForm(f=>({...f,description:e.target.value}))} placeholder="Mô tả ngắn..." /></label>
          <label style="grid-column:span 2">System Prompt<textarea style="${inp};min-height:80px;resize:vertical;font-family:var(--mono)" value=${form.system_prompt} onInput=${e=>setForm(f=>({...f,system_prompt:e.target.value}))} placeholder="You are a..." /></label>
        </div>
        <div style="margin-top:12px;display:flex;gap:8px;justify-content:flex-end">
          <button class="btn btn-outline" onClick=${()=>setShowForm(false)}>Huỷ</button>
          <button class="btn" style="background:var(--grad1);color:#fff;padding:8px 20px" onClick=${saveAgent}>💾 ${editAgent?'Cập nhật':'Tạo'}</button>
        </div>
      </div>
    `}

    <div class="card">${loading?html`<div style="text-align:center;padding:20px;color:var(--text2)">Loading...</div>`:agents.length===0?html`<div style="text-align:center;padding:30px;color:var(--text2)"><div style="font-size:48px;margin-bottom:12px">🤖</div><p>Default agent: <strong>${config?.agent_name||'BizClaw'}</strong></p><p style="margin-top:8px">Provider: <span class="badge badge-blue">${config?.default_provider||'—'}</span></p></div>`:html`
      <table><thead><tr><th>Agent</th><th>Vai trò</th><th>Provider</th><th>Model</th><th>Messages</th><th>Status</th><th style="text-align:right">Thao tác</th></tr></thead><tbody>
        ${agents.map(a=>html`<tr key=${a.name||a.id}>
          <td><strong>${a.name}</strong>${a.description?html`<div style="font-size:11px;color:var(--text2)">${a.description}</div>`:''}</td>
          <td><span class="badge">${a.role||'—'}</span></td>
          <td>${a.provider||'—'}</td>
          <td><span class="badge badge-blue">${a.model||'—'}</span></td>
          <td>${a.message_count||a.messages_processed||0}</td>
          <td><span class="badge badge-green">Active</span></td>
          <td style="text-align:right;white-space:nowrap">
            <button class="btn btn-outline btn-sm" onClick=${()=>openEdit(a)} title="Sửa">✏️</button>
            ${!a.is_default?html`<button class="btn btn-outline btn-sm" style="margin-left:4px;color:var(--red)" onClick=${()=>deleteAgent(a.name)} title="Xoá">🗑️</button>`:''}
          </td>
        </tr>`)}
      </tbody></table>
    `}</div>
  </div>`;
}

// ═══ KNOWLEDGE PAGE (with Add/Delete) ═══
function KnowledgePage({ lang }) {
  const { showToast } = useContext(AppContext);
  const [docs,setDocs] = useState([]);
  const [loading,setLoading] = useState(true);
  const [showAdd,setShowAdd] = useState(false);
  const [addForm,setAddForm] = useState({name:'',content:'',source:'upload'});
  const [uploading,setUploading] = useState(false);
  const [dragOver,setDragOver] = useState(false);

  const load = async () => {
    try{const r=await authFetch('/api/v1/knowledge/documents');const d=await r.json();setDocs(d.documents||[]);}catch(e){}
    setLoading(false);
  };
  useEffect(()=>{ load(); },[]);

  const addDoc = async () => {
    if(!addForm.name.trim()||!addForm.content.trim()) { showToast('⚠️ Nhập tên và nội dung','error'); return; }
    try {
      const r = await authFetch('/api/v1/knowledge/documents', {
        method:'POST', headers:{'Content-Type':'application/json'},
        body:JSON.stringify(addForm)
      });
      const d=await r.json();
      if(d.ok) { showToast('✅ Đã thêm: '+addForm.name+' ('+d.chunks+' chunks)','success'); setShowAdd(false); setAddForm({name:'',content:'',source:'upload'}); load(); }
      else showToast('❌ '+(d.error||'Lỗi'),'error');
    } catch(e) { showToast('❌ '+e.message,'error'); }
  };

  // Upload file (PDF, TXT, MD, etc.) via multipart
  const uploadFile = async (file) => {
    if (!file) return;
    const maxSize = 10 * 1024 * 1024; // 10MB limit
    if (file.size > maxSize) {
      showToast('❌ File quá lớn (tối đa 10MB)', 'error');
      return;
    }
    setUploading(true);
    try {
      const formData = new FormData();
      formData.append('file', file);
      const r = await authFetch('/api/v1/knowledge/upload', {
        method: 'POST',
        body: formData,
      });
      const d = await r.json();
      if (d.ok) {
        const sizeKB = Math.round((d.size || file.size) / 1024);
        showToast('✅ ' + d.name + ' → ' + d.chunks + ' chunks (' + sizeKB + 'KB)', 'success');
        load();
      } else {
        showToast('❌ ' + (d.error || 'Upload failed'), 'error');
      }
    } catch(e) {
      showToast('❌ Upload error: ' + e.message, 'error');
    }
    setUploading(false);
  };

  // Handle drag-and-drop
  const onDrop = (e) => {
    e.preventDefault();
    setDragOver(false);
    const files = e.dataTransfer?.files;
    if (files && files.length > 0) {
      for (let i = 0; i < files.length; i++) {
        uploadFile(files[i]);
      }
    }
  };

  const onDragOver = (e) => { e.preventDefault(); setDragOver(true); };
  const onDragLeave = () => { setDragOver(false); };

  // File picker
  const pickFile = () => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.pdf,.txt,.md,.json,.csv,.log,.toml,.yaml,.yml';
    input.multiple = true;
    input.onchange = (e) => {
      const files = e.target.files;
      for (let i = 0; i < files.length; i++) {
        uploadFile(files[i]);
      }
    };
    input.click();
  };

  const deleteDoc = async (id,name) => {
    if(!confirm('Xoá tài liệu "'+name+'"?')) return;
    try {
      const r = await authFetch('/api/v1/knowledge/documents/'+id, {method:'DELETE'});
      const d=await r.json();
      if(d.ok) { showToast('🗑️ Đã xoá: '+name,'success'); load(); }
      else showToast('❌ '+(d.error||'Lỗi'),'error');
    } catch(e) { showToast('❌ '+e.message,'error'); }
  };

  const inp = 'width:100%;padding:8px;margin-top:4px;background:var(--bg2);border:1px solid var(--border);border-radius:6px;color:var(--text);font-size:13px';

  const dropZoneStyle = dragOver
    ? 'border:2px dashed var(--accent);background:rgba(99,102,241,0.08);border-radius:12px;padding:32px;text-align:center;transition:all 0.2s;cursor:pointer'
    : 'border:2px dashed var(--border);background:var(--bg2);border-radius:12px;padding:32px;text-align:center;transition:all 0.2s;cursor:pointer';

  return html`<div>
    <div class="page-header"><div><h1>📚 ${t('kb.title',lang)}</h1><div class="sub">${t('kb.subtitle',lang)}</div></div>
      <div style="display:flex;gap:8px">
        <button class="btn" style="background:var(--grad1);color:#fff;padding:8px 18px" onClick=${pickFile}>📤 Upload File</button>
        <button class="btn btn-outline" onClick=${()=>setShowAdd(!showAdd)}>✏️ Paste Text</button>
      </div>
    </div>
    <div class="stats"><${StatsCard} label=${t('kb.documents',lang)} value=${docs.length} color="accent" icon="📄" /><${StatsCard} label=${t('kb.chunks',lang)} value=${docs.reduce((s,d)=>s+(d.chunks||0),0)} color="blue" icon="📝" /></div>

    ${html`<div class="card" style="margin-bottom:14px"
      onDrop=${onDrop} onDragOver=${onDragOver} onDragLeave=${onDragLeave}>
      <div style="${dropZoneStyle}">
        ${uploading ? html`
          <div style="font-size:32px;margin-bottom:8px">⏳</div>
          <div style="font-size:14px;color:var(--text2)">Đang xử lý...</div>
        ` : html`
          <div style="font-size:32px;margin-bottom:8px">${dragOver ? '📥' : '📄'}</div>
          <div style="font-size:14px;color:var(--text2)">Kéo thả file vào đây hoặc click <strong>Upload File</strong></div>
          <div style="margin-top:8px;display:flex;gap:6px;justify-content:center;flex-wrap:wrap">
            <span class="badge badge-green">PDF</span>
            <span class="badge badge-blue">TXT</span>
            <span class="badge badge-blue">MD</span>
            <span class="badge badge-blue">JSON</span>
            <span class="badge badge-blue">CSV</span>
          </div>
          <div style="margin-top:6px;font-size:11px;color:var(--text2)">Tối đa 10MB • PDF sẽ được trích xuất text tự động</div>
        `}
      </div>
    </div>`}

    ${showAdd && html`
      <div class="card" style="margin-bottom:14px;border:1px solid var(--accent)">
        <h3 style="margin-bottom:12px">✏️ Paste nội dung trực tiếp</h3>
        <div style="display:grid;gap:10px;font-size:13px">
          <label>Tên tài liệu<input style="${inp}" value=${addForm.name} onInput=${e=>setAddForm(f=>({...f,name:e.target.value}))} placeholder="guide.md, faq.txt..." /></label>
          <label>Nội dung<textarea style="${inp};min-height:200px;resize:vertical;font-family:var(--mono)" value=${addForm.content} onInput=${e=>setAddForm(f=>({...f,content:e.target.value}))} placeholder="Paste nội dung tài liệu vào đây... (Markdown, text, FAQ)" /></label>
        </div>
        <div style="margin-top:12px;display:flex;gap:8px;justify-content:flex-end">
          <button class="btn btn-outline" onClick=${()=>setShowAdd(false)}>Huỷ</button>
          <button class="btn" style="background:var(--grad1);color:#fff;padding:8px 20px" onClick=${addDoc}>💾 Thêm</button>
        </div>
      </div>
    `}

    <div class="card">${loading?html`<div style="text-align:center;padding:20px;color:var(--text2)">Loading...</div>`:docs.length===0?html`<div style="text-align:center;padding:40px;color:var(--text2)"><div style="font-size:48px;margin-bottom:12px">📚</div><p>Chưa có tài liệu. Upload PDF hoặc paste text để bắt đầu.</p></div>`:html`
      <table><thead><tr><th>Tài liệu</th><th>Chunks</th><th>Source</th><th style="text-align:right">Thao tác</th></tr></thead><tbody>
        ${docs.map(d=>html`<tr key=${d.id}><td><strong>${d.name && d.name.endsWith('.pdf') ? '📄 ' : '📝 '}${d.title||d.name}</strong></td><td>${d.chunks}</td><td style="font-size:12px">${d.source}</td>
          <td style="text-align:right"><button class="btn btn-outline btn-sm" style="color:var(--red)" onClick=${()=>deleteDoc(d.id,d.title||d.name)} title="Xoá">🗑️</button></td>
        </tr>`)}
      </tbody></table>
    `}</div>
  </div>`;
}

// (old MCP removed — new MCP with Add/Remove is above)

// ═══ ORCHESTRATION PAGE (with Create/Delete) ═══
function OrchestrationPage({ lang }) {
  const { showToast } = useContext(AppContext);
  const [delegations,setDelegations] = useState([]);
  const [links,setLinks] = useState([]);
  const [showCreate,setShowCreate] = useState(false);
  const [form,setForm] = useState({from:'',to:'',task:''});

  const load = async () => {
    try{const [r1,r2]=await Promise.all([authFetch('/api/v1/orchestration/delegations'),authFetch('/api/v1/orchestration/links')]);const d1=await r1.json();const d2=await r2.json();setDelegations(d1.delegations||[]);setLinks(d2.links||[]);}catch(e){}
  };
  useEffect(()=>{ load(); },[]);

  const createDelegation = async () => {
    if(!form.from.trim()||!form.to.trim()||!form.task.trim()) { showToast('⚠️ Điền đầy đủ','error'); return; }
    try {
      const r = await authFetch('/api/v1/orchestration/delegations', {method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify(form)});
      const d=await r.json();
      if(d.ok||d.id) { showToast('✅ Delegation created','success'); setShowCreate(false); setForm({from:'',to:'',task:''}); load(); }
      else showToast('❌ '+(d.error||'Lỗi'),'error');
    } catch(e) { showToast('❌ '+e.message,'error'); }
  };

  const deleteDelegation = async (id) => {
    if(!confirm('Xoá delegation?')) return;
    try { await authFetch('/api/v1/orchestration/delegations/'+id, {method:'DELETE'}); showToast('🗑️ Đã xoá','success'); load(); } catch(e) { showToast('❌ '+e.message,'error'); }
  };

  const inp = 'width:100%;padding:8px;margin-top:4px;background:var(--bg2);border:1px solid var(--border);border-radius:6px;color:var(--text);font-size:13px';

  return html`<div>
    <div class="page-header"><div><h1>🔀 ${t('orch.title',lang)}</h1><div class="sub">${t('orch.subtitle',lang)}</div></div>
      <button class="btn" style="background:var(--grad1);color:#fff;padding:8px 18px" onClick=${()=>setShowCreate(!showCreate)}>+ Tạo Delegation</button>
    </div>
    <div class="stats"><${StatsCard} label=${t('orch.delegations',lang)} value=${delegations.length} color="accent" icon="📋" /><${StatsCard} label=${t('orch.links',lang)} value=${links.length} color="blue" icon="🔗" /></div>
    ${showCreate && html`<div class="card" style="margin-bottom:14px;border:1px solid var(--accent)">
      <h3 style="margin-bottom:10px">📋 Tạo Delegation mới</h3>
      <div style="display:grid;grid-template-columns:1fr 1fr;gap:10px;font-size:13px">
        <label>From Agent<input style="${inp}" value=${form.from} onInput=${e=>setForm(f=>({...f,from:e.target.value}))} placeholder="main-agent" /></label>
        <label>To Agent<input style="${inp}" value=${form.to} onInput=${e=>setForm(f=>({...f,to:e.target.value}))} placeholder="research-agent" /></label>
        <label style="grid-column:span 2">Task<input style="${inp}" value=${form.task} onInput=${e=>setForm(f=>({...f,task:e.target.value}))} placeholder="Research topic X and report" /></label>
      </div>
      <div style="margin-top:12px;display:flex;gap:8px;justify-content:flex-end">
        <button class="btn btn-outline" onClick=${()=>setShowCreate(false)}>Huỷ</button>
        <button class="btn" style="background:var(--grad1);color:#fff;padding:8px 20px" onClick=${createDelegation}>📋 Delegate</button>
      </div>
    </div>`}
    <div style="display:grid;grid-template-columns:1fr 1fr;gap:14px">
      <div class="card"><h3 style="margin-bottom:12px">📋 ${t('orch.delegate_title',lang)}</h3>
        ${delegations.length===0?html`<div style="text-align:center;padding:20px;color:var(--text2)"><p>Chưa có delegation.</p></div>`:html`<table><thead><tr><th>${t('orch.from_agent',lang)}</th><th>${t('orch.to_agent',lang)}</th><th>${t('orch.task',lang)}</th><th>Status</th><th></th></tr></thead><tbody>${delegations.map(d=>html`<tr key=${d.id}><td>${d.from}</td><td>${d.to}</td><td style="max-width:200px;overflow:hidden;text-overflow:ellipsis">${d.task}</td><td><span class="badge badge-green">${d.status}</span></td><td><button class="btn btn-outline btn-sm" style="color:var(--red)" onClick=${()=>deleteDelegation(d.id)}>🗑️</button></td></tr>`)}</tbody></table>`}
      </div>
      <div class="card"><h3 style="margin-bottom:12px">🔗 ${t('orch.perm_links',lang)}</h3>
        <div style="display:grid;gap:8px">
          ${['delegate','handoff','broadcast','escalate'].map(p=>html`<div key=${p} style="display:flex;align-items:center;gap:10px;padding:8px 12px;background:var(--bg2);border-radius:6px">
            <span style="font-size:18px">${p==='delegate'?'📋':p==='handoff'?'🤝':p==='broadcast'?'📢':'⬆️'}</span>
            <div style="flex:1"><strong style="font-size:13px">${p}</strong><div style="font-size:11px;color:var(--text2)">Agent-to-agent ${p}</div></div>
            <span class="badge badge-green">✓ enabled</span>
          </div>`)}
        </div>
      </div>
    </div>
  </div>`;
}

// ═══ GALLERY PAGE (with Install) ═══
function GalleryPage({ lang }) {
  const { showToast } = useContext(AppContext);
  const [allSkills,setAllSkills] = useState([]);
  const [loading,setLoading] = useState(true);
  const [selectedCat,setSelectedCat] = useState(null);
  const [selectedSkill,setSelectedSkill] = useState(null);
  const [cloning,setCloning] = useState(false);
  const [search,setSearch] = useState('');

  useEffect(()=>{ (async()=>{ try{const r=await authFetch('/api/v1/gallery');const d=await r.json();setAllSkills(d.skills||[]);}catch(e){} setLoading(false); })(); },[]);

  const catMap = {
    hr:{icon:'🧑‍💼',label:'Nhân sự (HR)'},sales:{icon:'💰',label:'Kinh doanh'},finance:{icon:'📊',label:'Tài chính'},
    operations:{icon:'🏭',label:'Vận hành'},legal:{icon:'⚖️',label:'Pháp lý'},'customer-service':{icon:'📞',label:'CSKH'},
    marketing:{icon:'📣',label:'Marketing'},ecommerce:{icon:'🛒',label:'Thương mại ĐT'},management:{icon:'💼',label:'Quản lý'},
    admin:{icon:'📝',label:'Hành chính'},it:{icon:'💻',label:'IT'},analytics:{icon:'📧',label:'Phân tích'},
    training:{icon:'🎓',label:'Đào tạo'},productivity:{icon:'⚡',label:'Năng suất'}
  };

  const categories = [...new Set(allSkills.map(s=>s.cat))].filter(Boolean).sort();
  const catCounts = {};
  categories.forEach(c => { catCounts[c] = allSkills.filter(s=>s.cat===c).length; });

  const filtered = allSkills.filter(s => {
    if(selectedCat && s.cat !== selectedCat) return false;
    if(search) {
      const q = search.toLowerCase();
      return (s.name||'').toLowerCase().includes(q) || (s.desc||'').toLowerCase().includes(q) || (s.cat||'').toLowerCase().includes(q);
    }
    return true;
  });

  const cloneAsAgent = async (skill) => {
    setCloning(true);
    try {
      const r = await authFetch('/api/v1/agents', {
        method:'POST', headers:{'Content-Type':'application/json'},
        body:JSON.stringify({
          name: skill.id || skill.name.toLowerCase().replace(/\s+/g,'-'),
          role: skill.role || 'assistant',
          description: skill.desc || skill.name,
          system_prompt: skill.prompt || '',
          provider: '',
          model: ''
        })
      });
      const d = await r.json();
      if(d.ok) {
        showToast('✅ Đã tạo agent "'+skill.name+'" từ Gallery!','success');
        setSelectedSkill(null);
      } else {
        showToast('❌ '+(d.error||'Lỗi tạo agent'),'error');
      }
    } catch(e) { showToast('❌ '+e.message,'error'); }
    setCloning(false);
  };

  if(loading) return html`<div style="text-align:center;padding:60px;color:var(--text2)">⏳ Loading Gallery...</div>`;

  // Detail view
  if(selectedSkill) {
    const s = selectedSkill;
    return html`<div>
      <div class="page-header"><div><h1>📦 ${s.icon||'📦'} ${s.name}</h1><div class="sub">${s.desc}</div></div>
        <button class="btn btn-outline" onClick=${()=>setSelectedSkill(null)}>← Quay lại</button>
      </div>
      <div style="display:grid;grid-template-columns:1fr 1fr;gap:14px">
        <div class="card">
          <h3 style="margin-bottom:12px">📋 Thông tin</h3>
          <div style="display:grid;gap:8px;font-size:13px">
            <div style="display:flex;justify-content:space-between;padding:6px 10px;background:var(--bg2);border-radius:6px"><span style="color:var(--text2)">Danh mục</span><span class="badge badge-blue">${(catMap[s.cat]||{}).label||s.cat}</span></div>
            <div style="display:flex;justify-content:space-between;padding:6px 10px;background:var(--bg2);border-radius:6px"><span style="color:var(--text2)">Vai trò</span><span class="badge">${s.role||'assistant'}</span></div>
            <div style="display:flex;justify-content:space-between;padding:6px 10px;background:var(--bg2);border-radius:6px"><span style="color:var(--text2)">Tác giả</span><span>${s.author||'bizclaw'}</span></div>
            <div style="display:flex;justify-content:space-between;padding:6px 10px;background:var(--bg2);border-radius:6px"><span style="color:var(--text2)">ID</span><span style="font-family:var(--mono);font-size:12px">${s.id}</span></div>
          </div>
          <div style="margin-top:16px">
            <button class="btn" style="background:var(--grad1);color:#fff;padding:10px 24px;width:100%;font-size:14px" onClick=${()=>cloneAsAgent(s)} disabled=${cloning}>
              ${cloning ? '⏳ Đang tạo...' : '🤖 Clone thành Agent'}
            </button>
            <div style="font-size:11px;color:var(--text2);text-align:center;margin-top:6px">Tạo agent mới với System Prompt từ template này</div>
          </div>
        </div>
        <div class="card">
          <h3 style="margin-bottom:12px">💬 System Prompt</h3>
          <div style="padding:14px;background:var(--bg2);border-radius:8px;border:1px solid var(--border);font-size:13px;line-height:1.8;white-space:pre-wrap;max-height:400px;overflow-y:auto;font-family:var(--mono)">${s.prompt||'(Chưa có prompt)'}</div>
        </div>
      </div>
    </div>`;
  }

  return html`<div>
    <div class="page-header"><div><h1>📦 ${t('gallery.title',lang)}</h1><div class="sub">${t('gallery.subtitle',lang)} — ${allSkills.length} mẫu agent, ${categories.length} danh mục</div></div></div>
    <div class="stats">
      <${StatsCard} label="Templates" value=${allSkills.length} color="accent" icon="📦" />
      <${StatsCard} label="Danh mục" value=${categories.length} color="blue" icon="📁" />
      <${StatsCard} label=${selectedCat?(catMap[selectedCat]||{}).label||selectedCat:'Tất cả'} value=${filtered.length} color="green" icon="🔍" />
    </div>

    <div style="margin-bottom:14px"><input type="text" placeholder="🔍 Tìm template... (tên, mô tả, danh mục)" value=${search} onInput=${e=>setSearch(e.target.value)}
      style="width:100%;padding:10px 14px;background:var(--bg2);border:1px solid var(--border);border-radius:8px;color:var(--text);font-size:13px" /></div>

    <div class="card" style="margin-bottom:14px">
      <h3 style="margin-bottom:10px">📁 Danh mục ${selectedCat?html` — <span style="color:var(--accent)">${(catMap[selectedCat]||{}).label||selectedCat}</span> <button class="btn btn-outline btn-sm" style="margin-left:8px" onClick=${()=>setSelectedCat(null)}>✕ Xoá filter</button>`:''}</h3>
      <div style="display:flex;flex-wrap:wrap;gap:8px">
        ${categories.map(c=>html`<button key=${c} class="btn ${selectedCat===c?'':'btn-outline'} btn-sm" style="${selectedCat===c?'background:var(--grad1);color:#fff':''};display:flex;align-items:center;gap:4px"
          onClick=${()=>setSelectedCat(selectedCat===c?null:c)}>
          <span>${(catMap[c]||{}).icon||'📁'}</span> ${(catMap[c]||{}).label||c} <span class="badge" style="font-size:10px">${catCounts[c]}</span>
        </button>`)}
      </div>
    </div>

    <div class="card">
      <h3 style="margin-bottom:12px">🤖 Templates (${filtered.length})</h3>
      ${filtered.length===0?html`<div style="text-align:center;padding:30px;color:var(--text2)">Không tìm thấy template phù hợp.</div>`:html`
      <div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(300px,1fr));gap:10px">
        ${filtered.map(s=>html`<div key=${s.id||s.name} style="padding:12px 14px;background:var(--bg2);border-radius:8px;border:1px solid var(--border);cursor:pointer;transition:all 0.15s"
          onClick=${()=>setSelectedSkill(s)} onMouseOver=${e=>{e.currentTarget.style.borderColor='var(--accent)';e.currentTarget.style.transform='translateY(-1px)'}} onMouseOut=${e=>{e.currentTarget.style.borderColor='var(--border)';e.currentTarget.style.transform='none'}}>
          <div style="display:flex;align-items:center;gap:10px;margin-bottom:6px">
            <span style="font-size:28px">${s.icon||'📦'}</span>
            <div style="flex:1;min-width:0">
              <strong style="font-size:13px;display:block">${s.name}</strong>
              <span class="badge" style="font-size:10px;margin-top:2px">${(catMap[s.cat]||{}).label||s.cat}</span>
            </div>
            <button class="btn btn-outline btn-sm" onClick=${e=>{e.stopPropagation();cloneAsAgent(s)}} title="Clone thành Agent">🤖+</button>
          </div>
          <div style="font-size:12px;color:var(--text2);line-height:1.5;overflow:hidden;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical">${s.desc||''}</div>
        </div>`)}
      </div>`}
    </div>
  </div>`;
}

// ═══ BRAIN ENGINE PAGE (with Create/Edit/View files) ═══
function BrainPage({ lang }) {
  const { showToast } = useContext(AppContext);
  const [health,setHealth] = useState(null);
  const [files,setFiles] = useState([]);
  const [editFile,setEditFile] = useState(null);
  const [fileContent,setFileContent] = useState('');
  const [showNew,setShowNew] = useState(false);
  const [newName,setNewName] = useState('');

  const load = async () => {
    try{const r=await authFetch('/api/v1/health');setHealth(await r.json());}catch(e){}
    try{const r2=await authFetch('/api/v1/brain/files');const d2=await r2.json();setFiles(d2.files||[]);}catch(e){}
  };
  useEffect(()=>{ load(); },[]);

  const openFile = async (name) => {
    try {
      const r = await authFetch('/api/v1/brain/files/'+encodeURIComponent(name));
      const d = await r.json();
      setFileContent(d.content || '');
      setEditFile(name);
    } catch(e) { showToast('❌ '+e.message,'error'); }
  };

  const saveFile = async () => {
    try {
      const r = await authFetch('/api/v1/brain/files/'+encodeURIComponent(editFile), {
        method:'PUT', headers:{'Content-Type':'application/json'},
        body:JSON.stringify({content:fileContent})
      });
      const d = await r.json();
      if(d.ok) { showToast('✅ Đã lưu: '+editFile,'success'); load(); }
      else showToast('❌ '+(d.error||'Lỗi'),'error');
    } catch(e) { showToast('❌ '+e.message,'error'); }
  };

  const createFile = async () => {
    if(!newName.trim()) return;
    const fname = newName.endsWith('.md') ? newName : newName + '.md';
    try {
      const r = await authFetch('/api/v1/brain/files/'+encodeURIComponent(fname), {
        method:'PUT', headers:{'Content-Type':'application/json'},
        body:JSON.stringify({content:'# '+fname+'\n\n'})
      });
      const d = await r.json();
      if(d.ok) { showToast('✅ Đã tạo: '+fname,'success'); setShowNew(false); setNewName(''); load(); openFile(fname); }
      else showToast('❌ '+(d.error||'Lỗi'),'error');
    } catch(e) { showToast('❌ '+e.message,'error'); }
  };

  const checks = [
    {name:'SIMD (NEON/AVX)',status:health?.simd||'—',ok:true},{name:'Memory',status:health?.memory||'—',ok:true},
    {name:'Thread Pool',status:health?.threads||'—',ok:true},{name:'GGUF Parser',status:'ready',ok:true},
    {name:'KV Cache',status:'initialized',ok:true},{name:'Quantization',status:'Q4_K_M, Q5_K_M, Q8_0',ok:true},
  ];
  const inp = 'width:100%;padding:8px;margin-top:4px;background:var(--bg2);border:1px solid var(--border);border-radius:6px;color:var(--text);font-size:13px';

  return html`<div>
    <div class="page-header"><div><h1>🧠 ${t('brain.title',lang)}</h1><div class="sub">${t('brain.ws_sub',lang)}</div></div>
      <button class="btn" style="background:var(--grad1);color:#fff;padding:8px 18px" onClick=${()=>setShowNew(!showNew)}>+ Tạo file</button>
    </div>
    <div class="stats">
      <${StatsCard} label=${t('brain.engine',lang)} value="BizClaw Brain" color="accent" icon="🧠" />
      <${StatsCard} label=${t('brain.quant',lang)} value="Q4-Q8" color="blue" icon="📊" />
      <${StatsCard} label=${t('brain.files_count',lang)} value=${files.length} color="green" icon="📄" />
    </div>

    ${showNew && html`
      <div class="card" style="margin-bottom:14px;border:1px solid var(--accent)">
        <h3 style="margin-bottom:8px">📄 Tạo file mới</h3>
        <div style="display:flex;gap:8px;align-items:end">
          <label style="flex:1">Tên file<input style="${inp}" value=${newName} onInput=${e=>setNewName(e.target.value)} placeholder="MY_FILE.md" /></label>
          <button class="btn" style="background:var(--grad1);color:#fff;padding:8px 16px" onClick=${createFile}>Tạo</button>
        </div>
      </div>
    `}

    ${editFile && html`
      <div class="card" style="margin-bottom:14px;border:1px solid var(--accent)">
        <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:8px">
          <h3>📝 ${editFile}</h3>
          <div style="display:flex;gap:6px">
            <button class="btn" style="background:var(--grad1);color:#fff;padding:6px 16px" onClick=${saveFile}>💾 Lưu</button>
            <button class="btn btn-outline btn-sm" onClick=${()=>setEditFile(null)}>✕</button>
          </div>
        </div>
        <textarea value=${fileContent} onInput=${e=>setFileContent(e.target.value)}
          style="width:100%;min-height:300px;padding:12px;background:var(--bg2);border:1px solid var(--border);border-radius:8px;color:var(--text);font-family:var(--mono);font-size:13px;line-height:1.6;resize:vertical" />
      </div>
    `}

    <div style="display:grid;grid-template-columns:1fr 1fr;gap:14px">
      <div class="card"><h3 style="margin-bottom:12px">🏥 ${t('brain.health_title',lang)}</h3>
        <div style="display:grid;gap:6px">
          ${checks.map(c=>html`<div key=${c.name} style="display:flex;align-items:center;gap:8px;padding:8px 12px;background:var(--bg2);border-radius:6px">
            <span>${c.ok?'✅':'❌'}</span>
            <strong style="font-size:13px;flex:1">${c.name}</strong>
            <span style="font-size:12px;color:var(--text2)">${c.status}</span>
          </div>`)}
        </div>
      </div>
      <div class="card"><h3 style="margin-bottom:12px">📁 ${t('brain.ws_title',lang)}</h3>
        ${files.length===0?html`<div style="text-align:center;padding:20px;color:var(--text2)"><p>Workspace trống. Click "+ Tạo file" để bắt đầu.</p></div>`:html`<div style="display:grid;gap:4px">${files.map(f=>html`<div key=${f.name||f} style="display:flex;align-items:center;gap:8px;padding:6px 10px;background:var(--bg2);border-radius:4px;font-size:13px;cursor:pointer" onClick=${()=>openFile(f.name||f)} onMouseOver=${e=>e.currentTarget.style.borderColor='var(--accent)'} onMouseOut=${e=>e.currentTarget.style.borderColor='transparent'}>
          <span>📄</span><span style="flex:1">${f.name||f}</span><span style="color:var(--text2);font-size:11px">${f.size||''}</span>
          <span class="badge badge-blue" style="font-size:10px">✏️ Edit</span>
        </div>`)}</div>`}
      </div>
    </div>
  </div>`;
}

// ═══ CONFIG FILE PAGE (with actual Save) ═══
function ConfigFilePage({ lang }) {
  const { showToast } = useContext(AppContext);
  const [content,setContent] = useState('');
  const [loading,setLoading] = useState(true);
  useEffect(()=>{ (async()=>{ try{const r=await authFetch('/api/v1/config/full');const d=await r.json();setContent(d.content||d.raw||JSON.stringify(d,null,2)||'# config.toml not loaded');}catch(e){setContent('# Error loading config');} setLoading(false); })(); },[]);

  const save = async () => {
    try {
      const r = await authFetch('/api/v1/config/update', {
        method:'POST', headers:{'Content-Type':'application/json'},
        body: JSON.stringify({raw:content})
      });
      const d = await r.json();
      if(d.ok) showToast('✅ Config saved','success');
      else showToast('❌ '+(d.error||'Lỗi'),'error');
    } catch(e) { showToast('❌ '+e.message,'error'); }
  };

  return html`<div>
    <div class="page-header"><div><h1>📄 ${t('config.title',lang)}</h1><div class="sub">Xem và chỉnh sửa config.toml trực tiếp</div></div></div>
    <div class="card">
      <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px">
        <h3>📝 config.toml</h3>
        <button class="btn" style="background:var(--grad1);color:#fff;padding:6px 16px" onClick=${save}>💾 ${t('form.save',lang)}</button>
      </div>
      ${loading?html`<div style="text-align:center;padding:20px;color:var(--text2)">Loading...</div>`:html`
        <textarea value=${content} onInput=${e=>setContent(e.target.value)}
          style="width:100%;min-height:500px;padding:16px;background:var(--bg2);border:1px solid var(--border);border-radius:8px;color:var(--text);font-family:var(--mono);font-size:13px;line-height:1.6;resize:vertical;white-space:pre;overflow-x:auto" />
      `}
    </div>
  </div>`;
}

// ═══ LLM TRACES PAGE ═══
function TracesPage({ lang }) {
  const [traces, setTraces] = useState([]);
  const [stats, setStats] = useState({});
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    (async () => {
      try {
        const res = await authFetch('/api/v1/traces');
        const data = await res.json();
        setTraces(data.traces || []);
        setStats(data.stats || {});
      } catch (e) { console.error('Traces load:', e); }
      setLoading(false);
    })();
  }, []);

  const fmtLatency = (ms) => ms < 1000 ? ms + 'ms' : (ms / 1000).toFixed(1) + 's';
  const fmtCost = (c) => c < 0.001 ? '<$0.001' : '$' + c.toFixed(4);
  const fmtTime = (t) => new Date(t).toLocaleTimeString('en-US', { hour12: false });

  return html`<div>
    <div class="page-header"><div>
      <h1>📊 LLM Traces</h1>
      <div class="sub">Monitor every LLM call — tokens, latency, cost</div>
    </div></div>

    <div class="stats">
      <${StatsCard} label="Total Calls" value=${stats.total_calls || 0} color="accent" />
      <${StatsCard} label="Total Tokens" value=${(stats.total_tokens || 0).toLocaleString()} color="blue" />
      <${StatsCard} label="Avg Latency" value=${fmtLatency(stats.avg_latency_ms || 0)} color="green" />
      <${StatsCard} label="Total Cost" value=${fmtCost(stats.total_cost_usd || 0)} color="orange" />
      <${StatsCard} label="Cache Hit" value=${((stats.cache_hit_rate || 0) * 100).toFixed(0) + '%'} color="accent" />
    </div>

    <div class="card">
      <h3 style="margin-bottom:12px">📈 Recent Traces (${traces.length})</h3>
      ${loading ? html`<div style="text-align:center;padding:20px;color:var(--text2)">Loading...</div>` : html`
        <table>
          <thead><tr>
            <th>Time</th><th>Model</th><th>Prompt</th><th>Completion</th><th>Total</th>
            <th>Latency</th><th>Cost</th><th>Cache</th><th>Status</th>
          </tr></thead>
          <tbody>
            ${traces.map(t => html`<tr key=${t.id}>
              <td style="font-family:var(--mono);font-size:12px">${fmtTime(t.timestamp)}</td>
              <td><span class="badge badge-blue">${t.model}</span></td>
              <td style="font-family:var(--mono);font-size:12px">${t.prompt_tokens}</td>
              <td style="font-family:var(--mono);font-size:12px">${t.completion_tokens}</td>
              <td style="font-family:var(--mono);font-size:12px;font-weight:600">${t.total_tokens}</td>
              <td style="font-family:var(--mono);font-size:12px">${fmtLatency(t.latency_ms)}</td>
              <td style="font-family:var(--mono);font-size:12px;color:var(--orange)">${fmtCost(t.cost_usd)}</td>
              <td>${t.cache_hit ? '✅' : '➖'}</td>
              <td><span class="badge ${t.status === 'ok' ? 'badge-green' : 'badge-red'}">${t.status}</span></td>
            </tr>`)}
          </tbody>
        </table>
      `}
    </div>
  </div>`;
}

// ═══ COST TRACKING PAGE ═══
function CostPage({ lang }) {
  const [breakdown, setBreakdown] = useState([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    (async () => {
      try {
        const res = await authFetch('/api/v1/traces/cost');
        const data = await res.json();
        setBreakdown(data.breakdown || []);
        setTotal(data.total_cost_usd || 0);
      } catch (e) { console.error('Cost load:', e); }
      setLoading(false);
    })();
  }, []);

  const fmtCost = (c) => c < 0.001 ? '<$0.001' : '$' + c.toFixed(4);
  const sorted = [...breakdown].sort((a, b) => b.cost_usd - a.cost_usd);

  return html`<div>
    <div class="page-header"><div>
      <h1>💰 Cost Tracking</h1>
      <div class="sub">LLM cost breakdown by model (session)</div>
    </div></div>

    <div class="stats">
      <${StatsCard} label="Total Cost" value=${fmtCost(total)} color="orange" icon="💰" />
      <${StatsCard} label="Models Used" value=${breakdown.length} color="blue" icon="🤖" />
      <${StatsCard} label="Total Calls" value=${breakdown.reduce((s, b) => s + b.calls, 0)} color="accent" icon="📞" />
    </div>

    <div class="card">
      <h3 style="margin-bottom:12px">📊 Cost by Model</h3>
      ${loading ? html`<div style="text-align:center;padding:20px;color:var(--text2)">Loading...</div>` : html`
        <table>
          <thead><tr><th>Model</th><th>Calls</th><th>Tokens</th><th>Cost</th><th>% of Total</th></tr></thead>
          <tbody>
            ${sorted.map(b => html`<tr key=${b.model}>
              <td><span class="badge badge-blue">${b.model}</span></td>
              <td style="font-family:var(--mono)">${b.calls}</td>
              <td style="font-family:var(--mono)">${(b.total_tokens || 0).toLocaleString()}</td>
              <td style="font-family:var(--mono);color:var(--orange);font-weight:600">${fmtCost(b.cost_usd)}</td>
              <td>
                <div style="background:var(--bg2);border-radius:4px;height:16px;overflow:hidden">
                  <div style="background:var(--grad1);height:100%;width:${total > 0 ? (b.cost_usd / total * 100) : 0}%;border-radius:4px"></div>
                </div>
              </td>
            </tr>`)}
          </tbody>
        </table>
      `}
    </div>
  </div>`;
}

// ═══ ACTIVITY FEED PAGE ═══
function ActivityPage({ lang }) {
  const [events, setEvents] = useState([]);
  const [loading, setLoading] = useState(true);

  const loadEvents = async () => {
    try {
      const res = await authFetch('/api/v1/activity');
      const data = await res.json();
      setEvents(data.events || []);
    } catch (e) { console.error('Activity load:', e); }
    setLoading(false);
  };

  useEffect(() => {
    loadEvents();
    const timer = setInterval(loadEvents, 5000);
    return () => clearInterval(timer);
  }, []);

  const fmtTime = (t) => new Date(t).toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' });
  const typeIcon = (t) => {
    if (t.includes('llm')) return '🤖';
    if (t.includes('tool')) return '🛠️';
    if (t.includes('scheduler')) return '⏰';
    if (t.includes('channel')) return '📨';
    return '⚡';
  };
  const typeBadge = (t) => {
    if (t.includes('error')) return 'badge-red';
    if (t.includes('completed')) return 'badge-green';
    if (t.includes('started')) return 'badge-yellow';
    return 'badge-blue';
  };

  return html`<div>
    <div class="page-header"><div>
      <h1>⚡ Activity Feed</h1>
      <div class="sub">Real-time system events (auto-refreshes every 5s)</div>
    </div></div>

    <div class="stats">
      <${StatsCard} label="Events" value=${events.length} color="accent" icon="⚡" />
    </div>

    <div class="card">
      <h3 style="margin-bottom:12px">📝 Event Log</h3>
      ${loading ? html`<div style="text-align:center;padding:20px;color:var(--text2)">Loading...</div>` : events.length === 0
        ? html`<div style="text-align:center;padding:40px;color:var(--text2)">
            <div style="font-size:48px;margin-bottom:12px">🌟</div>
            <p>No activity yet. Start a conversation or run a scheduled task!</p>
          </div>`
        : html`<div style="display:flex;flex-direction:column;gap:8px">
            ${events.map(ev => html`
              <div key=${ev.timestamp} style="display:flex;align-items:center;gap:12px;padding:10px 14px;background:var(--bg2);border-radius:8px;border:1px solid var(--border)">
                <div style="font-size:20px">${typeIcon(ev.event_type)}</div>
                <div style="flex:1">
                  <div style="display:flex;align-items:center;gap:8px">
                    <span class="badge ${typeBadge(ev.event_type)}">${ev.event_type}</span>
                    <span style="color:var(--text2);font-size:12px">${ev.agent}</span>
                  </div>
                  <div style="font-size:13px;margin-top:4px">${ev.detail}</div>
                </div>
                <div style="font-family:var(--mono);font-size:11px;color:var(--text2)">${fmtTime(ev.timestamp)}</div>
              </div>
            `)}
          </div>`
      }
    </div>
  </div>`;
}

// ═══ WORKFLOWS PAGE (with Run/Delete) ═══
function WorkflowsPage({ lang }) {
  const { showToast } = useContext(AppContext);
  const [workflows, setWorkflows] = useState([]);
  const [loading, setLoading] = useState(true);
  const [selectedWf, setSelectedWf] = useState(null);

  useEffect(() => {
    (async () => {
      try {
        const r = await authFetch('/api/v1/workflows');
        const d = await r.json();
        setWorkflows(d.workflows || []);
      } catch (e) {
        // Use built-in templates as fallback
        setWorkflows([
          { id: 'content_pipeline', name: 'Content Pipeline', description: t('wf.content_desc', lang), tags: ['content','writing'], steps: [
            { name: 'Draft', type: 'Sequential', agent_role: 'Writer' },
            { name: 'Review', type: 'Sequential', agent_role: 'Editor' },
            { name: 'Polish', type: 'Sequential', agent_role: 'Proofreader' },
          ]},
          { id: 'expert_consensus', name: 'Expert Consensus', description: t('wf.expert_desc', lang), tags: ['analysis','multi-agent'], steps: [
            { name: 'Expert Analysis', type: 'FanOut', agent_role: '3 Experts (parallel)' },
            { name: 'Merge Results', type: 'Collect', agent_role: 'Synthesizer' },
          ]},
          { id: 'quality_pipeline', name: 'Quality Gate', description: t('wf.quality_desc', lang), tags: ['quality','loop'], steps: [
            { name: 'Generate', type: 'Sequential', agent_role: 'Creator' },
            { name: 'Evaluate', type: 'Loop', agent_role: 'Evaluator (until APPROVED)' },
          ]},
          { id: 'research_pipeline', name: 'Research Pipeline', description: t('wf.research_desc', lang), tags: ['research','data'], steps: [
            { name: 'Search', type: 'Sequential', agent_role: 'Researcher' },
            { name: 'Analyze', type: 'Sequential', agent_role: 'Analyst' },
            { name: 'Synthesize', type: 'Sequential', agent_role: 'Writer' },
            { name: 'Report', type: 'Transform', agent_role: 'Formatter' },
          ]},
          { id: 'translation_pipeline', name: 'Translation Pipeline', description: t('wf.translate_desc', lang), tags: ['language','translation'], steps: [
            { name: 'Translate', type: 'Sequential', agent_role: 'Translator' },
            { name: 'Verify Quality', type: 'Conditional', agent_role: 'QA Checker' },
          ]},
          { id: 'code_review', name: 'Code Review Pipeline', description: t('wf.codereview_desc', lang), tags: ['code','security'], steps: [
            { name: 'Code Analysis', type: 'FanOut', agent_role: '3 Reviewers (parallel)' },
            { name: 'Security Check', type: 'Sequential', agent_role: 'Security Auditor' },
            { name: 'Summary', type: 'Collect', agent_role: 'Lead Reviewer' },
          ]},
        ]);
      }
      setLoading(false);
    })();
  }, []);

  const stepTypeIcon = (type) => {
    const icons = { Sequential: '➡️', FanOut: '🔀', Collect: '📥', Conditional: '🔀', Loop: '🔁', Transform: '✨' };
    return icons[type] || '⚙️';
  };
  const stepTypeBadge = (type) => {
    const colors = { Sequential: 'badge-blue', FanOut: 'badge-purple', Collect: 'badge-green', Conditional: 'badge-orange', Loop: 'badge-yellow', Transform: 'badge-blue' };
    return colors[type] || 'badge-blue';
  };

  const runWorkflow = async (wf) => {
    try {
      const r = await authFetch('/api/v1/workflows/run', {method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify({workflow_id:wf.id, input:''})});
      const d=await r.json();
      if(d.ok) showToast('▶ Running: '+wf.name,'success');
      else showToast('❌ '+(d.error||'Lỗi'),'error');
    } catch(e) { showToast('❌ '+e.message,'error'); }
  };

  const deleteWorkflow = async (wf) => {
    if(!confirm('Xoá workflow "'+wf.name+'"?')) return;
    try {
      await authFetch('/api/v1/workflows/'+wf.id, {method:'DELETE'});
      showToast('🗑️ Đã xoá: '+wf.name,'success');
      setWorkflows(prev => prev.filter(w => w.id !== wf.id));
    } catch(e) { showToast('❌ '+e.message,'error'); }
  };

  return html`<div>
    <div class="page-header"><div>
      <h1>🔄 ${t('wf.title', lang)}</h1>
      <div class="sub">${t('wf.subtitle', lang)}</div>
    </div></div>

    <div class="stats">
      <${StatsCard} label=${t('wf.total', lang)} value=${workflows.length} color="accent" icon="🔄" />
      <${StatsCard} label=${t('wf.step_types', lang)} value="6" color="blue" icon="⚙️" />
      <${StatsCard} label=${t('wf.templates', lang)} value=${workflows.length} color="green" icon="📋" />
    </div>

    <div style="display:grid;grid-template-columns:1fr 1fr;gap:14px">
      <div class="card">
        <h3 style="margin-bottom:12px">⚙️ ${t('wf.step_types', lang)}</h3>
        <div style="display:grid;gap:6px">
          ${[['Sequential','➡️','Steps run one after another'],['FanOut','🔀','Multiple steps run in parallel'],['Collect','📥','Gather results (All/Best/Vote/Merge)'],['Conditional','🔀','If/else branching'],['Loop','🔁','Repeat until condition met'],['Transform','✨','Template transformation']].map(([name,icon,desc]) => html`
            <div key=${name} style="display:flex;align-items:center;gap:10px;padding:8px 12px;background:var(--bg2);border-radius:6px">
              <span style="font-size:20px">${icon}</span>
              <div style="flex:1"><strong style="font-size:13px">${name}</strong><div style="font-size:11px;color:var(--text2)">${desc}</div></div>
              <span class="badge ${stepTypeBadge(name)}">${name}</span>
            </div>
          `)}
        </div>
      </div>

      <div class="card">
        <h3 style="margin-bottom:12px">📋 ${t('wf.templates', lang)}</h3>
        ${loading ? html`<div style="text-align:center;padding:20px;color:var(--text2)">Loading...</div>` : html`
          <div style="display:grid;gap:8px">
            ${workflows.map(wf => html`<div key=${wf.id} style="padding:12px;background:var(--bg2);border-radius:8px;border:1px solid ${selectedWf===wf.id?'var(--accent)':'var(--border)'};cursor:pointer" onClick=${()=>setSelectedWf(selectedWf===wf.id?null:wf.id)}>
              <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:6px">
                <strong style="font-size:14px">${wf.name}</strong>
                <div style="display:flex;gap:4px;align-items:center">
                  ${(wf.tags||[]).map(tag=>html`<span key=${tag} class="badge" style="font-size:10px">${tag}</span>`)}
                  <button class="btn btn-outline btn-sm" onClick=${(e)=>{e.stopPropagation();runWorkflow(wf);}} title="Run">▶</button>
                  <button class="btn btn-outline btn-sm" style="color:var(--red)" onClick=${(e)=>{e.stopPropagation();deleteWorkflow(wf);}} title="Xoá">🗑️</button>
                </div>
              </div>
              <div style="font-size:12px;color:var(--text2);margin-bottom:8px">${wf.description}</div>
              ${selectedWf===wf.id && html`<div style="display:flex;gap:4px;flex-wrap:wrap;margin-top:8px;padding-top:8px;border-top:1px solid var(--border)">
                ${(wf.steps||[]).map((s,i)=>html`<div key=${i} style="display:flex;align-items:center;gap:4px;padding:4px 8px;background:var(--bg);border-radius:4px;font-size:11px">
                  <span>${stepTypeIcon(s.type)}</span>
                  <strong>${s.name}</strong>
                  <span style="color:var(--text2)">→ ${s.agent_role}</span>
                  ${i<wf.steps.length-1?html`<span style="margin-left:4px">→</span>`:''}
                </div>`)}
              </div>`}
            </div>`)}
          </div>
        `}
      </div>
    </div>
  </div>`;
}

// ═══ SKILLS MARKETPLACE PAGE (with Install/Uninstall) ═══
function SkillsPage({ lang }) {
  const { showToast } = useContext(AppContext);
  const [skills, setSkills] = useState([]);
  const [loading, setLoading] = useState(true);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState('all');

  useEffect(() => {
    (async () => {
      try {
        const r = await authFetch('/api/v1/skills');
        const d = await r.json();
        setSkills(d.skills || []);
      } catch (e) {
        setSkills([
          { name: 'Rust Expert', icon: '🦀', category: 'coding', tags: ['rust','systems','performance'], version: '1.0.0', description: t('skill.rust_desc', lang), installed: true },
          { name: 'Python Analyst', icon: '🐍', category: 'data', tags: ['python','pandas','visualization'], version: '1.0.0', description: t('skill.python_desc', lang), installed: true },
          { name: 'Web Developer', icon: '🌐', category: 'coding', tags: ['react','typescript','css'], version: '1.0.0', description: t('skill.web_desc', lang), installed: true },
          { name: 'DevOps Engineer', icon: '🔧', category: 'devops', tags: ['docker','k8s','ci-cd'], version: '1.0.0', description: t('skill.devops_desc', lang), installed: true },
          { name: 'Content Writer', icon: '✍️', category: 'writing', tags: ['blog','seo','marketing'], version: '1.0.0', description: t('skill.content_desc', lang), installed: true },
          { name: 'Security Auditor', icon: '🔒', category: 'security', tags: ['owasp','pentest','review'], version: '1.0.0', description: t('skill.security_desc', lang), installed: true },
          { name: 'SQL Expert', icon: '🗄️', category: 'data', tags: ['postgresql','sqlite','optimization'], version: '1.0.0', description: t('skill.sql_desc', lang), installed: true },
          { name: 'API Designer', icon: '🔌', category: 'coding', tags: ['rest','openapi','auth'], version: '1.0.0', description: t('skill.api_desc', lang), installed: true },
          { name: 'Vietnamese Business', icon: '🇻🇳', category: 'business', tags: ['tax','labor','accounting'], version: '1.0.0', description: t('skill.vnbiz_desc', lang), installed: true },
          { name: 'Git Workflow', icon: '📦', category: 'devops', tags: ['git','branching','review'], version: '1.0.0', description: t('skill.git_desc', lang), installed: true },
        ]);
      }
      setLoading(false);
    })();
  }, []);

  const categories = ['all','coding','data','devops','writing','security','business'];
  const catIcons = { all:'🌐', coding:'💻', data:'📊', devops:'🔧', writing:'✍️', security:'🔒', business:'💼' };

  const installSkill = async (name) => {
    try {
      const r = await authFetch('/api/v1/skills/install', {method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify({skill:name})});
      const d=await r.json();
      if(d.ok) { showToast('✅ Installed: '+name,'success'); setSkills(prev=>prev.map(s=>s.name===name?{...s,installed:true}:s)); }
      else showToast('❌ '+(d.error||'Lỗi'),'error');
    } catch(e) { setSkills(prev=>prev.map(s=>s.name===name?{...s,installed:true}:s)); showToast('✅ '+name+' installed','success'); }
  };

  const uninstallSkill = async (name) => {
    if(!confirm('Gỡ cài "'+name+'"?')) return;
    try {
      const r = await authFetch('/api/v1/skills/uninstall', {method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify({skill:name})});
      const d=await r.json();
      if(d.ok) { showToast('🗑️ Uninstalled: '+name,'success'); setSkills(prev=>prev.map(s=>s.name===name?{...s,installed:false}:s)); }
      else showToast('❌ '+(d.error||'Lỗi'),'error');
    } catch(e) { setSkills(prev=>prev.map(s=>s.name===name?{...s,installed:false}:s)); showToast('🗑️ '+name+' uninstalled','success'); }
  };

  const filtered = skills.filter(s => {
    if (selectedCategory !== 'all' && s.category !== selectedCategory) return false;
    if (searchQuery && !s.name.toLowerCase().includes(searchQuery.toLowerCase()) && !(s.tags||[]).some(t=>t.includes(searchQuery.toLowerCase()))) return false;
    return true;
  });

  return html`<div>
    <div class="page-header"><div>
      <h1>🧩 ${t('skill.title', lang)}</h1>
      <div class="sub">${t('skill.subtitle', lang)}</div>
    </div></div>

    <div class="stats">
      <${StatsCard} label=${t('skill.total', lang)} value=${skills.length} color="accent" icon="🧩" />
      <${StatsCard} label=${t('skill.installed', lang)} value=${skills.filter(s=>s.installed).length} color="green" icon="✅" />
      <${StatsCard} label=${t('skill.categories', lang)} value=${categories.length - 1} color="blue" icon="📁" />
    </div>

    <div class="card" style="margin-bottom:14px">
      <div style="display:flex;gap:10px;align-items:center;flex-wrap:wrap">
        <input placeholder=${t('skill.search', lang)} value=${searchQuery} onInput=${e=>setSearchQuery(e.target.value)}
          style="flex:1;min-width:200px;padding:10px 14px;background:var(--bg2);border:1px solid var(--border);border-radius:8px;color:var(--text);font-size:14px" />
        <div style="display:flex;gap:4px">
          ${categories.map(cat => html`<button key=${cat}
            class="btn ${selectedCategory===cat?'':'btn-outline'} btn-sm"
            style=${selectedCategory===cat?'background:var(--grad1);color:#fff':''}
            onClick=${()=>setSelectedCategory(cat)}>${catIcons[cat]} ${cat}</button>`)}
        </div>
      </div>
    </div>

    ${loading ? html`<div class="card" style="text-align:center;padding:40px;color:var(--text2)">Loading...</div>` : html`
      <div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(320px,1fr));gap:14px">
        ${filtered.map(skill => html`<div key=${skill.name} class="card" style="border-left:3px solid ${skill.installed?'var(--green)':'var(--border)'}">
          <div style="display:flex;align-items:center;gap:10px;margin-bottom:10px">
            <span style="font-size:32px">${skill.icon}</span>
            <div style="flex:1">
              <div style="display:flex;align-items:center;gap:6px">
                <strong style="font-size:15px">${skill.name}</strong>
                <span class="badge" style="font-size:10px">v${skill.version}</span>
              </div>
              <div style="font-size:11px;color:var(--text2)">${skill.category}</div>
            </div>
            ${skill.installed
              ? html`<button class="btn btn-sm" style="background:var(--green);color:#fff;font-size:11px" onClick=${()=>uninstallSkill(skill.name)}>✅ Gỡ cài</button>`
              : html`<button class="btn btn-outline btn-sm" onClick=${()=>installSkill(skill.name)}>+ ${t('skill.install', lang)}</button>`}
          </div>
          <div style="font-size:13px;color:var(--text2);margin-bottom:8px">${skill.description}</div>
          <div style="display:flex;gap:4px;flex-wrap:wrap">
            ${(skill.tags||[]).map(tag=>html`<span key=${tag} class="badge" style="font-size:10px">#${tag}</span>`)}
          </div>
        </div>`)}
      </div>
    `}
  </div>`;
}


// ═══ MAIN APP ═══
// ═══ WIKI & GUIDE PAGE ═══
const WIKI_ARTICLES = [
  {id:'getting-started',icon:'🚀',title:'Bắt đầu',content:'<h2>🚀 Bắt đầu sử dụng</h2><p>Dashboard này giúp bạn quản lý AI Agent. Các tính năng chính:</p><ul><li><strong>Chat:</strong> Trò chuyện trực tiếp với AI Agent</li><li><strong>Agents:</strong> Tạo và quản lý nhiều agent</li><li><strong>Channels:</strong> Kết nối Telegram, Zalo, Discord</li><li><strong>Knowledge:</strong> Thêm tài liệu cho AI</li><li><strong>Scheduler:</strong> Lên lịch tự động</li><li><strong>Gallery:</strong> 50+ mẫu agent template</li></ul><h3>Bước đầu tiên</h3><ol><li>Vào <strong>Settings</strong> để kiểm tra provider & model</li><li>Vào <strong>Chat</strong> để thử nói chuyện</li><li>Vào <strong>Channels</strong> để kết nối messaging</li></ol>'},
  {id:'chat-guide',icon:'💬',title:'Chat với Agent',content:'<h2>💬 Chat với Agent</h2><p>Trang Chat cho phép trò chuyện trực tiếp với AI Agent qua web.</p><h3>Cách dùng</h3><ol><li>Click <strong>Chat</strong> trên sidebar</li><li>Chọn agent trong sidebar (nếu có nhiều)</li><li>Nhập tin nhắn và nhấn Enter</li></ol><h3>Tính năng</h3><ul><li><strong>Multi-agent:</strong> Chọn agent khác nhau</li><li><strong>History:</strong> Lịch sử tự lưu</li><li><strong>Markdown:</strong> Code blocks, lists, tables</li><li><strong>Streaming:</strong> Response word-by-word</li></ul>'},
  {id:'channels-guide',icon:'📱',title:'Kênh liên lạc',content:'<h2>📱 Cấu hình kênh</h2><p>Kết nối agent với messaging.</p><h3>Telegram</h3><ol><li>Mở @BotFather → /newbot → Copy Token</li><li>Vào Channels → Bật Telegram</li><li>Paste Bot Token → Lưu</li></ol><h3>Zalo OA</h3><ol><li>Tạo OA tại oa.zalo.me</li><li>Lấy App ID, Secret Key, Access Token</li><li>Điền form → Lưu</li></ol><h3>Khác</h3><ul><li><strong>Discord:</strong> Bot Token</li><li><strong>Email:</strong> IMAP/SMTP</li><li><strong>Webhook:</strong> Custom endpoint</li></ul>'},
  {id:'knowledge-guide',icon:'📚',title:'Kho tri thức',content:'<h2>📚 Kho tri thức (RAG)</h2><p>Thêm tài liệu để AI trả lời chính xác hơn.</p><h3>Thêm tài liệu</h3><ol><li>Vào Kho tri thức → "+ Thêm tài liệu"</li><li>Upload hoặc paste nội dung</li><li>Lưu — hệ thống tự chia chunks</li></ol><h3>Best Practices</h3><ul><li>Upload FAQ, product catalog, SOP</li><li>Chia tài liệu dài thành nhiều file</li><li>Dùng tiêu đề rõ ràng</li></ul>'},
  {id:'scheduler-guide',icon:'⏰',title:'Lịch trình',content:'<h2>⏰ Lịch trình tự động</h2><p>Agent tự chạy prompt theo lịch.</p><h3>Tạo tác vụ</h3><ol><li>Vào Lịch trình → "+ Thêm tác vụ"</li><li>Chọn Agent, nhập Prompt</li><li>Nhập Cron expression</li><li>Chọn kênh nhận kết quả</li></ol><h3>Cron cheat sheet</h3><p><code>0 9 * * *</code> = 9:00 mỗi ngày<br><code>*/30 * * * *</code> = mỗi 30 phút<br><code>0 8 * * 1</code> = 8:00 T2</p>'},
  {id:'agents-guide',icon:'🤖',title:'Multi-Agent',content:'<h2>🤖 Quản lý Agent</h2><p>Tạo nhiều agent với vai trò khác nhau.</p><h3>Tạo Agent</h3><ol><li>Vào AI Agent → "+ Tạo Agent"</li><li>Đặt tên, vai trò, provider/model</li><li>Viết System Prompt</li></ol><h3>Gán kênh</h3><p>Click ✏️ Sửa → "Gán Agent với Kênh" → chọn kênh.</p><h3>Gallery Skills</h3><p>Vào Gallery duyệt 50+ template theo ngành.</p>'}
];

function WikiPage({ lang }) {
  const [activeId, setActiveId] = useState('getting-started');
  const [searchQ, setSearchQ] = useState('');
  const [showSearch, setShowSearch] = useState(false);

  const article = WIKI_ARTICLES.find(a => a.id === activeId) || WIKI_ARTICLES[0];
  const results = searchQ ? WIKI_ARTICLES.filter(a =>
    a.title.toLowerCase().includes(searchQ.toLowerCase()) ||
    a.content.toLowerCase().includes(searchQ.toLowerCase())
  ) : null;

  return html`
    <div class="page-header"><div><h1>📖 Wiki & Hướng dẫn</h1><div class="sub">Tài liệu hướng dẫn sử dụng hệ thống</div></div>
      <button class="btn btn-outline btn-sm" onclick=${() => setShowSearch(!showSearch)}>🔍 Tìm kiếm</button>
    </div>
    ${showSearch && html`<div style="margin-bottom:16px"><input type="text" placeholder="Tìm kiếm..." value=${searchQ} onInput=${e => setSearchQ(e.target.value)} style="width:100%;padding:10px 14px;background:var(--bg2);border:1px solid var(--border);border-radius:8px;color:var(--text);font-size:13px" /></div>`}
    <div style="display:grid;grid-template-columns:200px 1fr;gap:16px">
      <div class="card" style="position:sticky;top:20px;align-self:start">
        <div style="font-size:12px;font-weight:600;color:var(--accent);margin-bottom:10px">📑 Mục lục</div>
        ${WIKI_ARTICLES.map(a => html`
          <a href="#" onclick=${e => { e.preventDefault(); setActiveId(a.id); setSearchQ(''); }}
            style="display:block;padding:3px 6px;border-radius:4px;text-decoration:none;font-size:12px;line-height:2;color:${activeId===a.id?'var(--accent)':'var(--text)'};background:${activeId===a.id?'var(--bg2)':'transparent'};font-weight:${activeId===a.id?'600':'400'}">${a.icon} ${a.title}</a>
        `)}
      </div>
      <div class="card" style="min-height:400px;font-size:13px;line-height:1.8" dangerouslySetInnerHTML=${{ __html: results ? (results.length ? '<h2>🔍 '+results.length+' kết quả</h2>' + results.map(a => '<div class="card" style="margin:8px 0;cursor:pointer" onclick=""><strong>'+a.icon+' '+a.title+'</strong></div>').join('') : '<p style="color:var(--text2);text-align:center;padding:30px">Không tìm thấy</p>') : article.content }} />
    </div>
  `;
}

// ═══ AI CHAT WIDGET ═══
function ChatWidget() {
  const [open, setOpen] = useState(false);
  const [msgs, setMsgs] = useState([
    { from:'bot', text:'👋 Chào bạn! Hỏi về: Chat, Channels, Agent, Lịch trình, Kho tri thức...' }
  ]);
  const [input, setInput] = useState('');
  const msgsRef = useRef(null);
  const { navigate } = useContext(AppContext);

  const send = () => {
    if (!input.trim()) return;
    const q = input.trim();
    setMsgs(prev => [...prev, { from:'user', text: q }]);
    setInput('');
    const lq = q.toLowerCase();
    let best = null, bestScore = 0;
    WIKI_ARTICLES.forEach(a => {
      let score = 0;
      const hay = (a.title+' '+a.content).toLowerCase();
      lq.split(/\s+/).forEach(k => { if(hay.includes(k)) score++; });
      if(score > bestScore) { bestScore = score; best = a; }
    });
    setTimeout(() => {
      if(best && bestScore >= 1) {
        const snippet = best.content.replace(/<[^>]+>/g,'').slice(0,200);
        setMsgs(prev => [...prev, { from:'bot', text: `📖 ${best.icon} ${best.title}\n\n${snippet}...\n\n→ Xem Wiki để biết thêm` }]);
      } else {
        setMsgs(prev => [...prev, { from:'bot', text: '🤔 Thử hỏi: chat, telegram, agent, lịch trình...' }]);
      }
    }, 300);
  };

  useEffect(() => { if(msgsRef.current) msgsRef.current.scrollTop = msgsRef.current.scrollHeight; }, [msgs]);

  return html`
    <div style="position:fixed;bottom:20px;right:20px;z-index:9999">
      ${open && html`
        <div style="width:360px;height:480px;background:var(--surface);border:1px solid var(--border);border-radius:14px;box-shadow:0 8px 32px rgba(0,0,0,0.4);display:flex;flex-direction:column;overflow:hidden;margin-bottom:10px">
          <div style="padding:12px 16px;background:linear-gradient(135deg,var(--accent),#7c3aed);color:#fff;display:flex;justify-content:space-between;align-items:center;border-radius:14px 14px 0 0">
            <div><strong>🤖 Trợ lý</strong><div style="font-size:10px;opacity:0.8">Hỏi cách sử dụng</div></div>
            <button onclick=${() => setOpen(false)} style="background:none;border:none;color:#fff;font-size:16px;cursor:pointer">✕</button>
          </div>
          <div ref=${msgsRef} style="flex:1;overflow-y:auto;padding:12px;display:flex;flex-direction:column;gap:8px">
            ${msgs.map(m => html`
              <div style="background:${m.from==='user'?'var(--accent)':'var(--bg2)'};color:${m.from==='user'?'#fff':'var(--text)'};padding:8px 12px;border-radius:10px;font-size:12px;line-height:1.6;max-width:85%;align-self:${m.from==='user'?'flex-end':'flex-start'};white-space:pre-wrap">${m.text}</div>
            `)}
          </div>
          <div style="padding:8px 12px;border-top:1px solid var(--border);display:flex;gap:6px">
            <input value=${input} onInput=${e => setInput(e.target.value)} onKeyDown=${e => e.key==='Enter' && send()} placeholder="Hỏi gì đó..." style="flex:1;padding:7px 10px;background:var(--bg2);border:1px solid var(--border);border-radius:6px;color:var(--text);font-size:12px" />
            <button onclick=${send} class="btn btn-primary btn-sm">📤</button>
          </div>
        </div>
      `}
      <button onclick=${() => setOpen(!open)} style="width:48px;height:48px;border-radius:50%;background:linear-gradient(135deg,var(--accent),#7c3aed);border:none;color:#fff;font-size:20px;cursor:pointer;box-shadow:0 4px 16px rgba(99,102,241,0.4);transition:transform 0.2s" onmouseenter="this.style.transform='scale(1.1)'" onmouseleave="this.style.transform='scale(1)'">💬</button>
    </div>
  `;
}

export function App() {
  // Read current page from URL for initial load
  const initPage = location.pathname.replace(/^\//, '').replace(/\/$/, '') || 'dashboard';
  const [currentPage, setCurrentPage] = useState(initPage);
  const [lang, setLang] = useState(localStorage.getItem('bizclaw_lang') || 'vi');
  const [wsStatus, setWsStatus] = useState('disconnected');
  const [config, setConfig] = useState({});
  const [toast, setToast] = useState(null);
  const [paired, setPaired] = useState(false);
  const [checkingPairing, setCheckingPairing] = useState(true);
  const wsRef = useRef(null);

  // Check pairing
  useEffect(() => {
    (async () => {
      try {
        const res = await fetch('/api/v1/verify-pairing', {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ code: pairingCode || '' })
        });
        const r = await res.json();
        if (r.ok) { setPaired(true); }
        else if (pairingCode) {
          // Try stored code
          const res2 = await fetch('/api/v1/verify-pairing', {
            method: 'POST', headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ code: pairingCode })
          });
          const r2 = await res2.json();
          if (r2.ok) setPaired(true);
        }
      } catch (e) { setPaired(true); } // if API fails, assume no pairing required
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
    if (!paired) return;
    let reconnectAttempts = 0;
    let pingTimer = null;

    function connect() {
      const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
      const codeParam = pairingCode ? '?code=' + encodeURIComponent(pairingCode) : '';
      const socket = new WebSocket(proto + '//' + location.host + '/ws' + codeParam);

      socket.onopen = () => {
        reconnectAttempts = 0;
        setWsStatus('connected');
        pingTimer = setInterval(() => {
          if (socket.readyState === 1) socket.send(JSON.stringify({ type: 'ping' }));
        }, 25000);
      };
      socket.onclose = () => {
        setWsStatus('disconnected');
        if (pingTimer) clearInterval(pingTimer);
        reconnectAttempts++;
        const delay = Math.min(1000 * Math.pow(1.5, reconnectAttempts), 30000);
        setTimeout(connect, delay);
      };
      socket.onerror = (e) => console.error('WS error:', e);
      socket.onmessage = (e) => {
        try {
          const msg = JSON.parse(e.data);
          // Handle WS messages (for chat)
          window.dispatchEvent(new CustomEvent('ws-message', { detail: msg }));
        } catch (err) { console.error('WS parse:', err); }
      };
      wsRef.current = socket;
      window._ws = socket;
    }
    connect();

    return () => {
      if (wsRef.current) wsRef.current.close();
      if (pingTimer) clearInterval(pingTimer);
    };
  }, [paired]);

  // History API: handle browser back/forward
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

  // Show toast
  const showToast = useCallback((msg, type = 'info') => {
    setToast({ message: msg, type });
    setTimeout(() => setToast(null), 3000);
  }, []);
  window.showToast = showToast;

  if (checkingPairing) return html`<div style="display:flex;align-items:center;justify-content:center;height:100vh;background:var(--bg);color:var(--text2)">⏳ Loading...</div>`;
  if (!paired) return html`<${PairingGate} onSuccess=${() => setPaired(true)} />`;

  // Navigate: use manual render() to swap page content.
  // Preact+HTM's vdom diffing has a bug where <main> content is not updated
  // even when state changes. This nuclear fix bypasses vdom entirely.
  const mainRef = useRef(null);
  
  const doRenderPage = useCallback((pageId) => {
    if (!mainRef.current) return;
    const cfg = window.__bizclaw_config || config;
    const ln = window.__bizclaw_lang || lang;
    let pageVNode;
    switch (pageId) {
      case 'dashboard': pageVNode = html`<${DashboardPage} config=${cfg} lang=${ln} />`; break;
      case 'chat': pageVNode = html`<${ChatPage} config=${cfg} lang=${ln} />`; break;
      case 'hands': pageVNode = html`<${HandsPage} lang=${ln} />`; break;
      case 'settings': pageVNode = html`<${SettingsPage} config=${cfg} lang=${ln} />`; break;
      case 'providers': pageVNode = html`<${ProvidersPage} config=${cfg} lang=${ln} />`; break;
      case 'channels': pageVNode = html`<${ChannelsPage} lang=${ln} />`; break;
      case 'tools': pageVNode = html`<${ToolsPage} lang=${ln} />`; break;
      case 'agents': pageVNode = html`<${AgentsPage} config=${cfg} lang=${ln} />`; break;
      case 'knowledge': pageVNode = html`<${KnowledgePage} lang=${ln} />`; break;
      case 'mcp': pageVNode = html`<${McpPage} lang=${ln} />`; break;
      case 'orchestration': pageVNode = html`<${OrchestrationPage} lang=${ln} />`; break;
      case 'gallery': pageVNode = html`<${GalleryPage} lang=${ln} />`; break;
      case 'brain': pageVNode = html`<${SettingsPage} config=${cfg} lang=${ln} />`; break;
      case 'configfile': pageVNode = html`<${ConfigFilePage} lang=${ln} />`; break;
      case 'scheduler': pageVNode = html`<${SchedulerPage} lang=${ln} />`; break;
      case 'traces': pageVNode = html`<${TracesPage} lang=${ln} />`; break;
      case 'cost': pageVNode = html`<${CostPage} lang=${ln} />`; break;
      case 'activity': pageVNode = html`<${ActivityPage} lang=${ln} />`; break;
      case 'workflows': pageVNode = html`<${WorkflowsPage} lang=${ln} />`; break;
      case 'skills': pageVNode = html`<${SkillsPage} lang=${ln} />`; break;
      case 'wiki': pageVNode = html`<${WikiPage} lang=${ln} />`; break;
      default: pageVNode = html`<div class="card" style="padding:40px;text-align:center"><div style="font-size:48px;margin-bottom:16px">📄</div><h2>${pageId}</h2></div>`; break;
    }
    // Manual render into <main> — bypasses parent vdom diffing entirely
    render(pageVNode, mainRef.current);
  }, [config, lang]);

  // Render initial page when mainRef is available
  useEffect(() => {
    if (mainRef.current) doRenderPage(currentPage);
  }, [currentPage, doRenderPage]);

  // Keep globals in sync for doRenderPage closure
  window.__bizclaw_config = config;
  window.__bizclaw_lang = lang;

  function navigate(pageId) {
    const path = '/' + (pageId === 'dashboard' ? '' : pageId);
    if (location.pathname !== path) {
      history.pushState({}, '', path);
    }
    setCurrentPage(pageId);
    // Also trigger immediate render (don't wait for state update)
    setTimeout(() => doRenderPage(pageId), 0);
  }

  // Global navigate — must be set AFTER navigate is created
  window._navigate = navigate;

  return html`
    <${AppContext.Provider} value=${{ config, lang, t: (k) => t(k, lang), showToast, navigate, wsStatus }}>
      <div class="app">
        <${Sidebar}
          currentPage=${currentPage}
          onNavigate=${navigate}
          lang=${lang}
          onLangChange=${changeLang}
          wsStatus=${wsStatus}
          agentName=${config?.agent_name || 'BizClaw Agent'}
        />
        <main class="main" ref=${mainRef}>
        </main>
      </div>
      <${Toast} ...${toast || {}} />
      <${ChatWidget} />
    <//>
  `;
}

// Dedicated page router component — Preact properly diffs props and re-renders
// when 'page' changes, unlike inline switch/renderPage() inside HTM templates.
function PageRouter({ page, config, lang }) {
  switch (page) {
    case 'dashboard': return html`<${DashboardPage} config=${config} lang=${lang} />`;
    case 'chat': return html`<${ChatPage} config=${config} lang=${lang} />`;
    case 'hands': return html`<${HandsPage} lang=${lang} />`;
    case 'settings': return html`<${SettingsPage} config=${config} lang=${lang} />`;
    case 'providers': return html`<${ProvidersPage} config=${config} lang=${lang} />`;
    case 'channels': return html`<${ChannelsPage} lang=${lang} />`;
    case 'tools': return html`<${ToolsPage} lang=${lang} />`;
    case 'agents': return html`<${AgentsPage} config=${config} lang=${lang} />`;
    case 'knowledge': return html`<${KnowledgePage} lang=${lang} />`;
    case 'mcp': return html`<${McpPage} lang=${lang} />`;
    case 'orchestration': return html`<${OrchestrationPage} lang=${lang} />`;
    case 'gallery': return html`<${GalleryPage} lang=${lang} />`;
    case 'brain': return html`<${SettingsPage} config=${config} lang=${lang} />`;
    case 'configfile': return html`<${ConfigFilePage} lang=${lang} />`;
    case 'scheduler': return html`<${SchedulerPage} lang=${lang} />`;
    case 'traces': return html`<${TracesPage} lang=${lang} />`;
    case 'cost': return html`<${CostPage} lang=${lang} />`;
    case 'activity': return html`<${ActivityPage} lang=${lang} />`;
    case 'workflows': return html`<${WorkflowsPage} lang=${lang} />`;
    case 'skills': return html`<${SkillsPage} lang=${lang} />`;
    case 'wiki': return html`<${WikiPage} lang=${lang} />`;
    default: return html`<div class="card" style="padding:40px;text-align:center"><div style="font-size:48px;margin-bottom:16px">📄</div><h2>${page}</h2></div>`;
  }
}
