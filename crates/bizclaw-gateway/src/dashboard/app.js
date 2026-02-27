// BizClaw Dashboard — Main App Component
// Preact + HTM, no build step required
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
  { id: 'hands', icon: '🤚', label: 'Autonomous Hands' },
  { id: 'settings', icon: '⚙️', label: 'nav.settings' },
  { id: 'providers', icon: '🔌', label: 'nav.providers' },
  { id: 'channels', icon: '📱', label: 'nav.channels' },
  { id: 'tools', icon: '🛠️', label: 'nav.tools' },
  { id: 'mcp', icon: '🔗', label: 'nav.mcp' },
  { id: 'agents', icon: '🤖', label: 'nav.agents' },
  { id: 'orchestration', icon: '🔀', label: 'nav.orchestration' },
  { id: 'gallery', icon: '📦', label: 'nav.gallery' },
  { id: 'knowledge', icon: '📚', label: 'nav.knowledge' },
  { id: 'scheduler', icon: '⏰', label: 'nav.scheduler' },
  { id: 'traces', icon: '📊', label: 'LLM Traces' },
  { id: 'cost', icon: '💰', label: 'Cost Tracking' },
  { id: 'activity', icon: '⚡', label: 'Activity Feed' },
  { id: 'sep2', sep: true },
  { id: 'brain', icon: '🧠', label: 'nav.brain' },
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
          ${['chat', 'settings', 'channels', 'brain', 'configfile'].map(p => html`
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

// ═══ AUTONOMOUS HANDS PAGE ═══
function HandsPage({ lang }) {
  const hands = [
    { name:'research', icon:'🔍', label:'Research Hand', schedule:'Every 6h', phases:['gather','analyze','report'], status:'idle', runs:0, tokens:0, cost:0 },
    { name:'analytics', icon:'📊', label:'Analytics Hand', schedule:'Daily 6:00', phases:['collect','process','report'], status:'idle', runs:0, tokens:0, cost:0 },
    { name:'content', icon:'📝', label:'Content Hand', schedule:'Daily 8:00', phases:['ideate','create','review ⚠️'], status:'idle', runs:0, tokens:0, cost:0 },
    { name:'monitor', icon:'🔔', label:'Monitor Hand', schedule:'Every 5min', phases:['check','alert'], status:'idle', runs:0, tokens:0, cost:0 },
    { name:'sync', icon:'🔄', label:'Sync Hand', schedule:'Every 30min', phases:['fetch','reconcile','push ⚠️'], status:'idle', runs:0, tokens:0, cost:0 },
    { name:'outreach', icon:'📧', label:'Outreach Hand', schedule:'Weekdays 9AM', phases:['prepare','review ⚠️','send'], status:'idle', runs:0, tokens:0, cost:0 },
    { name:'security', icon:'🛡️', label:'Security Hand', schedule:'Every 1h', phases:['scan','analyze','report'], status:'idle', runs:0, tokens:0, cost:0 },
  ];
  const [enabled, setEnabled] = useState({research:true,analytics:true,content:true,monitor:true,sync:true,outreach:true,security:true});
  const toggle = (n) => setEnabled(p => ({...p,[n]:!p[n]}));
  const statusBadge = (s,en) => !en ? html`<span class="badge badge-purple">🚫 disabled</span>` : html`<span class="badge badge-green">⏹ idle</span>`;
  return html`<div>
    <div class="page-header"><div>
      <h1>🤚 Autonomous Hands</h1>
      <div class="sub">7 tay tự trị chạy 24/7 — lấy cảm hứng từ kiến trúc Agent OS</div>
    </div></div>
    <div class="stats">
      <\${StatsCard} label="Total Hands" value=${hands.length} color="accent" icon="🤚" />
      <\${StatsCard} label="Active" value=${Object.values(enabled).filter(Boolean).length} color="green" icon="▶" />
      <\${StatsCard} label="Total Runs" value="0" color="blue" icon="🔁" />
      <\${StatsCard} label="Total Cost" value="$0.00" color="orange" icon="💰" />
    </div>
    <div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(340px,1fr));gap:14px">
      ${hands.map(h => html`<div class="card" key=${h.name} style="border-left:3px solid ${enabled[h.name]?'var(--green)':'var(--text2)'}">
        <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:10px">
          <div style="display:flex;align-items:center;gap:8px">
            <span style="font-size:24px">${h.icon}</span>
            <div><strong>${h.label}</strong><div style="font-size:11px;color:var(--text2)">${h.schedule}</div></div>
          </div>
          <div style="display:flex;align-items:center;gap:6px">
            ${statusBadge(h.status,enabled[h.name])}
            <button class="btn btn-outline btn-sm" onClick=${()=>toggle(h.name)}>${enabled[h.name]?'⏸':'▶'}</button>
          </div>
        </div>
        <div style="display:flex;gap:4px;flex-wrap:wrap;margin-bottom:8px">
          ${h.phases.map((p,i) => html`<span key=${i} class="badge ${p.includes('⚠️')?'badge-orange':'badge-blue'}" style="font-size:10px">${i+1}. ${p}</span>`)}
        </div>
        <div style="display:grid;grid-template-columns:1fr 1fr 1fr;gap:6px;font-size:11px;color:var(--text2)">
          <div>Runs: <strong style="color:var(--text)">${h.runs}</strong></div>
          <div>Tokens: <strong style="color:var(--text)">${h.tokens}</strong></div>
          <div>Cost: <strong style="color:var(--orange)">$${h.cost.toFixed(4)}</strong></div>
        </div>
      </div>`)}
    </div>
  </div>`;
}

// ═══ SETTINGS PAGE ═══
function SettingsPage({ config, lang }) {
  const [form, setForm] = useState({provider:config?.default_provider||'',model:'',agentName:config?.agent_name||'',persona:'',temperature:0.7,autonomy:'supervised'});
  return html`<div>
    <div class="page-header"><div><h1>⚙️ ${t('settings.title',lang)}</h1><div class="sub">${t('settings.subtitle',lang)}</div></div></div>
    <div style="display:grid;grid-template-columns:1fr 1fr;gap:14px">
      <div class="card"><div class="card-label">🤖 ${t('set.provider_section',lang)}</div>
        <div style="display:grid;gap:10px;font-size:13px">
          <label>${t('set.provider',lang)}<input style="width:100%;padding:8px;margin-top:4px;background:var(--bg2);border:1px solid var(--border);border-radius:6px;color:var(--text)" value=${form.provider} onInput=${e=>setForm(f=>({...f,provider:e.target.value}))} /></label>
          <label>${t('set.model',lang)}<input style="width:100%;padding:8px;margin-top:4px;background:var(--bg2);border:1px solid var(--border);border-radius:6px;color:var(--text)" value=${form.model} onInput=${e=>setForm(f=>({...f,model:e.target.value}))} /></label>
          <label>${t('set.temperature',lang)}: ${form.temperature}<input type="range" min="0" max="2" step="0.1" value=${form.temperature} onInput=${e=>setForm(f=>({...f,temperature:+e.target.value}))} style="width:100%" /></label>
        </div>
      </div>
      <div class="card"><div class="card-label">🪪 ${t('set.identity',lang)}</div>
        <div style="display:grid;gap:10px;font-size:13px">
          <label>${t('set.agent_name',lang)}<input style="width:100%;padding:8px;margin-top:4px;background:var(--bg2);border:1px solid var(--border);border-radius:6px;color:var(--text)" value=${form.agentName} onInput=${e=>setForm(f=>({...f,agentName:e.target.value}))} /></label>
          <label>${t('set.persona',lang)}<input style="width:100%;padding:8px;margin-top:4px;background:var(--bg2);border:1px solid var(--border);border-radius:6px;color:var(--text)" value=${form.persona} /></label>
          <label>${t('set.autonomy',lang)}<select style="width:100%;padding:8px;margin-top:4px;background:var(--bg2);border:1px solid var(--border);border-radius:6px;color:var(--text)" value=${form.autonomy}>
            <option value="readonly">${t('set.readonly',lang)}</option><option value="supervised">${t('set.supervised',lang)}</option><option value="full">${t('set.full',lang)}</option>
          </select></label>
        </div>
      </div>
    </div>
    <div style="margin-top:14px;text-align:right"><button class="btn" style="background:var(--grad1);color:#fff;padding:10px 24px">${t('settings.save',lang)}</button></div>
  </div>`;
}

// ═══ PROVIDERS PAGE ═══
function ProvidersPage({ config, lang }) {
  const [providers, setProviders] = useState([]);
  const [loading, setLoading] = useState(true);
  useEffect(()=>{ (async()=>{ try { const r=await authFetch('/api/v1/providers'); const d=await r.json(); setProviders(d.providers||[]); } catch(e){} setLoading(false); })(); },[]);
  const active = config?.default_provider || '';
  const typeColor = t => t==='cloud'?'badge-blue':t==='local'?'badge-green':'badge-purple';
  return html`<div>
    <div class="page-header"><div><h1>🔌 ${t('providers.title',lang)}</h1><div class="sub">${t('providers.subtitle',lang)}</div></div></div>
    <div class="stats">
      <\${StatsCard} label=${t('providers.active_label',lang)} value=${active||'—'} color="green" icon="⚡" />
      <\${StatsCard} label=${t('providers.total_label',lang)} value=${providers.length} color="accent" icon="🔌" />
    </div>
    <div class="card">${loading?html`<div style="text-align:center;padding:20px;color:var(--text2)">Loading...</div>`:html`<table><thead><tr><th></th><th>Provider</th><th>Type</th><th>Models</th><th>Status</th><th></th></tr></thead><tbody>
      ${providers.map(p=>html`<tr key=${p.name}><td style="font-size:20px">${p.icon||'🤖'}</td><td><strong>${p.label||p.name}</strong></td><td><span class="badge ${typeColor(p.provider_type)}">${p.provider_type}</span></td><td style="font-size:12px">${(p.models||[]).slice(0,3).join(', ')}</td><td>${p.name===active?html`<span class="badge badge-green">✅ Active</span>`:html`<span class="badge">—</span>`}</td><td><button class="btn btn-outline btn-sm" onClick=${()=>window.showToast&&window.showToast('Provider '+p.name+' activated','success')}>⚡</button></td></tr>`)}
    </tbody></table>`}</div>
  </div>`;
}

// ═══ CHANNELS PAGE ═══
function ChannelsPage({ lang }) {
  const channels = [
    {name:'cli',icon:'💻',status:'active'},{name:'telegram',icon:'📱',status:'configured'},{name:'discord',icon:'🎮',status:'configured'},
    {name:'email',icon:'📧',status:'configured'},{name:'webhook',icon:'🌐',status:'active'},{name:'whatsapp',icon:'💬',status:'available'},
    {name:'zalo',icon:'💙',status:'configured'},{name:'slack',icon:'💬',status:'available'},{name:'line',icon:'📱',status:'available'},
    {name:'teams',icon:'🏢',status:'available'},{name:'signal',icon:'🔐',status:'available'},{name:'matrix',icon:'🔲',status:'available'},
    {name:'viber',icon:'💜',status:'available'},{name:'messenger',icon:'💙',status:'available'},{name:'mattermost',icon:'🔵',status:'available'},
    {name:'google_chat',icon:'🟢',status:'available'},{name:'dingtalk',icon:'🔷',status:'available'},{name:'feishu',icon:'🐦',status:'available'},
    {name:'mastodon',icon:'🐘',status:'available'},{name:'bluesky',icon:'🦋',status:'available'},{name:'nostr',icon:'🟣',status:'available'},
    {name:'twitter',icon:'🐦',status:'available'},{name:'twilio_sms',icon:'📱',status:'available'},{name:'xmpp',icon:'💬',status:'available'},
    {name:'webex',icon:'🌐',status:'available'},
  ];
  const statusBadge = s => s==='active'?html`<span class="badge badge-green">● Active</span>`:s==='configured'?html`<span class="badge badge-blue">✓ Configured</span>`:html`<span class="badge">○ Available</span>`;
  return html`<div>
    <div class="page-header"><div><h1>📱 ${t('channels.title',lang)}</h1><div class="sub">${t('channels.subtitle',lang)} — 25+ nền tảng</div></div></div>
    <div class="stats">
      <\${StatsCard} label="Total Channels" value=${channels.length} color="accent" />
      <\${StatsCard} label="Active" value=${channels.filter(c=>c.status==='active').length} color="green" />
      <\${StatsCard} label="Configured" value=${channels.filter(c=>c.status==='configured').length} color="blue" />
    </div>
    <div class="card"><div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(200px,1fr));gap:10px">
      ${channels.map(c=>html`<div key=${c.name} style="display:flex;align-items:center;gap:10px;padding:10px 14px;background:var(--bg2);border-radius:8px;border:1px solid var(--border)">
        <span style="font-size:22px">${c.icon}</span>
        <div style="flex:1"><strong style="font-size:13px">${c.name}</strong></div>
        ${statusBadge(c.status)}
      </div>`)}
    </div></div>
  </div>`;
}

// ═══ TOOLS PAGE ═══
function ToolsPage({ lang }) {
  const tools = [
    {name:'shell',icon:'🖥️',desc:t('tool.shell_desc',lang)},{name:'file',icon:'📁',desc:t('tool.file_desc',lang)},
    {name:'edit_file',icon:'✏️',desc:t('tool.editfile_desc',lang)},{name:'glob',icon:'🔍',desc:t('tool.glob_desc',lang)},
    {name:'grep',icon:'🔎',desc:t('tool.grep_desc',lang)},{name:'http_request',icon:'🌐',desc:t('tool.httpreq_desc',lang)},
    {name:'execute_code',icon:'⚡',desc:t('tool.execcode_desc',lang)},{name:'web_search',icon:'🔍',desc:'DuckDuckGo, SearXNG'},
    {name:'plan',icon:'📋',desc:t('tool.plan_desc',lang)},{name:'session_context',icon:'📊',desc:t('tool.sessionctx_desc',lang)},
    {name:'config_manager',icon:'⚙️',desc:t('tool.configmgr_desc',lang)},{name:'memory_search',icon:'🧠',desc:t('tool.memsearch_desc',lang)},
    {name:'doc_reader',icon:'📄',desc:t('tool.docreader_desc',lang)},
  ];
  return html`<div>
    <div class="page-header"><div><h1>🛠️ ${t('tools.title',lang)}</h1><div class="sub">${t('tools.subtitle',lang)}</div></div></div>
    <div class="stats"><\${StatsCard} label="Native Tools" value=${tools.length} color="accent" icon="🛠️" /><\${StatsCard} label="MCP Tools" value="∞" color="blue" icon="🔗" /></div>
    <div class="card"><div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(280px,1fr));gap:10px">
      ${tools.map(t=>html`<div key=${t.name} style="display:flex;align-items:flex-start;gap:10px;padding:12px;background:var(--bg2);border-radius:8px;border:1px solid var(--border)">
        <span style="font-size:24px">${t.icon}</span>
        <div><strong style="font-size:13px">${t.name}</strong><div style="font-size:11px;color:var(--text2);margin-top:2px">${t.desc}</div></div>
        <span class="badge badge-green" style="margin-left:auto">✓</span>
      </div>`)}
    </div></div>
  </div>`;
}

// ═══ AGENTS PAGE ═══
function AgentsPage({ config, lang }) {
  const [agents,setAgents] = useState([]);
  useEffect(()=>{ (async()=>{ try{const r=await authFetch('/api/v1/agents');const d=await r.json();setAgents(d.agents||[]);}catch(e){}})(); },[]);
  return html`<div>
    <div class="page-header"><div><h1>🤖 ${t('agents.title',lang)}</h1><div class="sub">${t('agents.subtitle',lang)}</div></div></div>
    <div class="stats"><\${StatsCard} label=${t('agents.total',lang)} value=${agents.length||1} color="accent" icon="🤖" /></div>
    <div class="card">${agents.length===0?html`<div style="text-align:center;padding:30px;color:var(--text2)"><div style="font-size:48px;margin-bottom:12px">🤖</div><p>Default agent: <strong>${config?.agent_name||'BizClaw'}</strong></p><p style="margin-top:8px">Provider: <span class="badge badge-blue">${config?.default_provider||'—'}</span></p></div>`:html`<table><thead><tr><th>Agent</th><th>Provider</th><th>Model</th><th>Messages</th><th>Status</th></tr></thead><tbody>${agents.map(a=>html`<tr key=${a.id}><td><strong>${a.name}</strong></td><td>${a.provider}</td><td><span class="badge badge-blue">${a.model}</span></td><td>${a.message_count||0}</td><td><span class="badge badge-green">Active</span></td></tr>`)}</tbody></table>`}</div>
  </div>`;
}

// ═══ KNOWLEDGE PAGE ═══
function KnowledgePage({ lang }) {
  const [docs,setDocs] = useState([]);
  useEffect(()=>{ (async()=>{ try{const r=await authFetch('/api/v1/knowledge/documents');const d=await r.json();setDocs(d.documents||[]);}catch(e){}})(); },[]);
  return html`<div>
    <div class="page-header"><div><h1>📚 ${t('kb.title',lang)}</h1><div class="sub">${t('kb.subtitle',lang)}</div></div></div>
    <div class="stats"><\${StatsCard} label=${t('kb.documents',lang)} value=${docs.length} color="accent" icon="📄" /><\${StatsCard} label=${t('kb.chunks',lang)} value=${docs.reduce((s,d)=>s+(d.chunks||0),0)} color="blue" icon="📝" /></div>
    <div class="card">${docs.length===0?html`<div style="text-align:center;padding:40px;color:var(--text2)"><div style="font-size:48px;margin-bottom:12px">📚</div><p>Chưa có tài liệu. Thêm file MD, PDF, TXT để AI trả lời chính xác hơn.</p></div>`:html`<table><thead><tr><th>Document</th><th>Chunks</th><th>Source</th></tr></thead><tbody>${docs.map(d=>html`<tr key=${d.id}><td><strong>${d.title}</strong></td><td>${d.chunks}</td><td style="font-size:12px">${d.source}</td></tr>`)}</tbody></table>`}</div>
  </div>`;
}

// ═══ MCP SERVERS PAGE ═══
function McpPage({ lang }) {
  const [servers,setServers] = useState([]);
  useEffect(()=>{ (async()=>{ try{const r=await authFetch('/api/v1/mcp/servers');const d=await r.json();setServers(d.servers||[]);}catch(e){}})(); },[]);
  const popular = [
    {name:'filesystem',desc:'Read/write filesystem',icon:'📁'},{name:'github',desc:'GitHub API',icon:'🐙'},
    {name:'postgres',desc:'PostgreSQL queries',icon:'🐘'},{name:'slack',desc:'Slack integration',icon:'💬'},
    {name:'puppeteer',desc:'Browser automation',icon:'🌐'},{name:'memory',desc:'Knowledge graph',icon:'🧠'},
    {name:'gdrive',desc:'Google Drive',icon:'📂'},{name:'sqlite',desc:'SQLite database',icon:'💾'},
  ];
  return html`<div>
    <div class="page-header"><div><h1>🔗 ${t('mcp.title',lang)}</h1><div class="sub">${t('mcp.subtitle',lang)}</div></div></div>
    <div class="stats">
      <\${StatsCard} label=${t('mcp.total',lang)} value=${servers.length} color="accent" icon="🔗" />
      <\${StatsCard} label=${t('mcp.active',lang)} value=${servers.filter(s=>s.status==='connected').length} color="green" icon="✅" />
    </div>
    <div class="card"><h3 style="margin-bottom:12px">🔌 ${t('mcp.popular',lang)}</h3>
      <div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(220px,1fr));gap:10px">
        ${popular.map(p=>html`<div key=${p.name} style="display:flex;align-items:center;gap:10px;padding:10px 14px;background:var(--bg2);border-radius:8px;border:1px solid var(--border)">
          <span style="font-size:22px">${p.icon}</span>
          <div style="flex:1"><strong style="font-size:13px">${p.name}</strong><div style="font-size:11px;color:var(--text2)">${p.desc}</div></div>
          <button class="btn btn-outline btn-sm">+</button>
        </div>`)}
      </div>
    </div>
    ${servers.length>0&&html`<div class="card" style="margin-top:14px"><h3 style="margin-bottom:12px">📡 Connected Servers (${servers.length})</h3>
      <table><thead><tr><th>Server</th><th>Protocol</th><th>Tools</th><th>Status</th></tr></thead><tbody>
        ${servers.map(s=>html`<tr key=${s.name}><td><strong>${s.name}</strong></td><td><span class="badge badge-blue">${s.transport||'stdio'}</span></td><td>${s.tools_count||0}</td><td><span class="badge ${s.status==='connected'?'badge-green':'badge-red'}">${s.status}</span></td></tr>`)}
      </tbody></table>
    </div>`}
  </div>`;
}

// ═══ ORCHESTRATION PAGE ═══
function OrchestrationPage({ lang }) {
  const [delegations,setDelegations] = useState([]);
  const [links,setLinks] = useState([]);
  useEffect(()=>{ (async()=>{ try{const [r1,r2]=await Promise.all([authFetch('/api/v1/orchestration/delegations'),authFetch('/api/v1/orchestration/links')]);const d1=await r1.json();const d2=await r2.json();setDelegations(d1.delegations||[]);setLinks(d2.links||[]);}catch(e){}})(); },[]);
  return html`<div>
    <div class="page-header"><div><h1>🔀 ${t('orch.title',lang)}</h1><div class="sub">${t('orch.subtitle',lang)}</div></div></div>
    <div class="stats">
      <\${StatsCard} label=${t('orch.delegations',lang)} value=${delegations.length} color="accent" icon="📋" />
      <\${StatsCard} label=${t('orch.links',lang)} value=${links.length} color="blue" icon="🔗" />
    </div>
    <div style="display:grid;grid-template-columns:1fr 1fr;gap:14px">
      <div class="card"><h3 style="margin-bottom:12px">📋 ${t('orch.delegate_title',lang)}</h3>
        ${delegations.length===0?html`<div style="text-align:center;padding:20px;color:var(--text2)"><p>Chưa có delegation. Dùng lệnh /delegate trong chat.</p></div>`:html`<table><thead><tr><th>${t('orch.from_agent',lang)}</th><th>${t('orch.to_agent',lang)}</th><th>${t('orch.task',lang)}</th><th>Status</th></tr></thead><tbody>${delegations.map(d=>html`<tr key=${d.id}><td>${d.from}</td><td>${d.to}</td><td style="max-width:200px;overflow:hidden;text-overflow:ellipsis">${d.task}</td><td><span class="badge badge-green">${d.status}</span></td></tr>`)}</tbody></table>`}
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

// ═══ GALLERY PAGE ═══
function GalleryPage({ lang }) {
  const [templates,setTemplates] = useState([]);
  useEffect(()=>{ (async()=>{ try{const r=await authFetch('/api/v1/gallery');const d=await r.json();setTemplates(d.templates||[]);}catch(e){}})(); },[]);
  const categories = [
    {name:'Kinh doanh',icon:'💼',count:12},{name:'Marketing',icon:'📈',count:8},{name:'Kỹ thuật',icon:'⚙️',count:10},
    {name:'Hỗ trợ',icon:'🎧',count:6},{name:'Giáo dục',icon:'📚',count:5},{name:'Sáng tạo',icon:'🎨',count:5},
    {name:'Tài chính',icon:'💰',count:3},{name:'Y tế',icon:'🏥',count:2},
  ];
  return html`<div>
    <div class="page-header"><div><h1>📦 ${t('gallery.title',lang)}</h1><div class="sub">${t('gallery.subtitle',lang)}</div></div></div>
    <div class="stats">
      <\${StatsCard} label="Templates" value=${templates.length||51} color="accent" icon="📦" />
      <\${StatsCard} label="Categories" value=${categories.length} color="blue" icon="📁" />
    </div>
    <div class="card"><h3 style="margin-bottom:12px">📁 Danh mục</h3>
      <div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(180px,1fr));gap:10px">
        ${categories.map(c=>html`<div key=${c.name} style="display:flex;align-items:center;gap:10px;padding:12px 14px;background:var(--bg2);border-radius:8px;border:1px solid var(--border);cursor:pointer" onMouseOver=${e=>e.currentTarget.style.borderColor='var(--accent)'} onMouseOut=${e=>e.currentTarget.style.borderColor='var(--border)'}>
          <span style="font-size:28px">${c.icon}</span>
          <div><strong>${c.name}</strong><div style="font-size:11px;color:var(--text2)">${c.count} templates</div></div>
        </div>`)}
      </div>
    </div>
  </div>`;
}

// ═══ BRAIN ENGINE PAGE ═══
function BrainPage({ lang }) {
  const [health,setHealth] = useState(null);
  const [files,setFiles] = useState([]);
  useEffect(()=>{ (async()=>{ try{const r=await authFetch('/api/v1/health');setHealth(await r.json());}catch(e){} try{const r2=await authFetch('/api/v1/brain/files');const d2=await r2.json();setFiles(d2.files||[]);}catch(e){}})(); },[]);
  const checks = [
    {name:'SIMD (NEON/AVX)',status:health?.simd||'—',ok:true},{name:'Memory',status:health?.memory||'—',ok:true},
    {name:'Thread Pool',status:health?.threads||'—',ok:true},{name:'GGUF Parser',status:'ready',ok:true},
    {name:'KV Cache',status:'initialized',ok:true},{name:'Quantization',status:'Q4_K_M, Q5_K_M, Q8_0',ok:true},
  ];
  return html`<div>
    <div class="page-header"><div><h1>🧠 ${t('brain.title',lang)}</h1><div class="sub">${t('brain.ws_sub',lang)}</div></div></div>
    <div class="stats">
      <\${StatsCard} label=${t('brain.engine',lang)} value="BizClaw Brain" color="accent" icon="🧠" />
      <\${StatsCard} label=${t('brain.quant',lang)} value="Q4-Q8" color="blue" icon="📊" />
      <\${StatsCard} label=${t('brain.files_count',lang)} value=${files.length} color="green" icon="📄" />
    </div>
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
        ${files.length===0?html`<div style="text-align:center;padding:20px;color:var(--text2)"><p>Workspace trống. Các tệp persona, knowledge sẽ hiển thị ở đây.</p></div>`:html`<div style="display:grid;gap:4px">${files.map(f=>html`<div key=${f.name} style="display:flex;align-items:center;gap:8px;padding:6px 10px;background:var(--bg2);border-radius:4px;font-size:13px">
          <span>📄</span><span style="flex:1">${f.name}</span><span style="color:var(--text2);font-size:11px">${f.size||''}</span>
        </div>`)}</div>`}
      </div>
    </div>
  </div>`;
}

// ═══ CONFIG FILE PAGE ═══
function ConfigFilePage({ lang }) {
  const [content,setContent] = useState('');
  const [loading,setLoading] = useState(true);
  useEffect(()=>{ (async()=>{ try{const r=await authFetch('/api/v1/config/full');const d=await r.json();setContent(d.content||d.raw||JSON.stringify(d,null,2)||'# config.toml not loaded');}catch(e){setContent('# Error loading config');} setLoading(false); })(); },[]);
  return html`<div>
    <div class="page-header"><div><h1>📄 ${t('config.title',lang)}</h1><div class="sub">Xem và chỉnh sửa config.toml trực tiếp</div></div></div>
    <div class="card">
      <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px">
        <h3>📝 config.toml</h3>
        <button class="btn" style="background:var(--grad1);color:#fff;padding:6px 16px" onClick=${()=>window.showToast&&window.showToast('Config saved','success')}>💾 ${t('form.save',lang)}</button>
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
      <\${StatsCard} label="Total Calls" value=\${stats.total_calls || 0} color="accent" />
      <\${StatsCard} label="Total Tokens" value=\${(stats.total_tokens || 0).toLocaleString()} color="blue" />
      <\${StatsCard} label="Avg Latency" value=\${fmtLatency(stats.avg_latency_ms || 0)} color="green" />
      <\${StatsCard} label="Total Cost" value=\${fmtCost(stats.total_cost_usd || 0)} color="orange" />
      <\${StatsCard} label="Cache Hit" value=\${((stats.cache_hit_rate || 0) * 100).toFixed(0) + '%'} color="accent" />
    </div>

    <div class="card">
      <h3 style="margin-bottom:12px">📈 Recent Traces (\${traces.length})</h3>
      \${loading ? html\`<div style="text-align:center;padding:20px;color:var(--text2)">Loading...</div>\` : html\`
        <table>
          <thead><tr>
            <th>Time</th><th>Model</th><th>Prompt</th><th>Completion</th><th>Total</th>
            <th>Latency</th><th>Cost</th><th>Cache</th><th>Status</th>
          </tr></thead>
          <tbody>
            \${traces.map(t => html\`<tr key=\${t.id}>
              <td style="font-family:var(--mono);font-size:12px">\${fmtTime(t.timestamp)}</td>
              <td><span class="badge badge-blue">\${t.model}</span></td>
              <td style="font-family:var(--mono);font-size:12px">\${t.prompt_tokens}</td>
              <td style="font-family:var(--mono);font-size:12px">\${t.completion_tokens}</td>
              <td style="font-family:var(--mono);font-size:12px;font-weight:600">\${t.total_tokens}</td>
              <td style="font-family:var(--mono);font-size:12px">\${fmtLatency(t.latency_ms)}</td>
              <td style="font-family:var(--mono);font-size:12px;color:var(--orange)">\${fmtCost(t.cost_usd)}</td>
              <td>\${t.cache_hit ? '✅' : '➖'}</td>
              <td><span class="badge \${t.status === 'ok' ? 'badge-green' : 'badge-red'}">\${t.status}</span></td>
            </tr>\`)}
          </tbody>
        </table>
      \`}
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
      <\${StatsCard} label="Total Cost" value=\${fmtCost(total)} color="orange" icon="💰" />
      <\${StatsCard} label="Models Used" value=\${breakdown.length} color="blue" icon="🤖" />
      <\${StatsCard} label="Total Calls" value=\${breakdown.reduce((s, b) => s + b.calls, 0)} color="accent" icon="📞" />
    </div>

    <div class="card">
      <h3 style="margin-bottom:12px">📊 Cost by Model</h3>
      \${loading ? html\`<div style="text-align:center;padding:20px;color:var(--text2)">Loading...</div>\` : html\`
        <table>
          <thead><tr><th>Model</th><th>Calls</th><th>Tokens</th><th>Cost</th><th>% of Total</th></tr></thead>
          <tbody>
            \${sorted.map(b => html\`<tr key=\${b.model}>
              <td><span class="badge badge-blue">\${b.model}</span></td>
              <td style="font-family:var(--mono)">\${b.calls}</td>
              <td style="font-family:var(--mono)">\${(b.total_tokens || 0).toLocaleString()}</td>
              <td style="font-family:var(--mono);color:var(--orange);font-weight:600">\${fmtCost(b.cost_usd)}</td>
              <td>
                <div style="background:var(--bg2);border-radius:4px;height:16px;overflow:hidden">
                  <div style="background:var(--grad1);height:100%;width:\${total > 0 ? (b.cost_usd / total * 100) : 0}%;border-radius:4px"></div>
                </div>
              </td>
            </tr>\`)}
          </tbody>
        </table>
      \`}
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
      <\${StatsCard} label="Events" value=\${events.length} color="accent" icon="⚡" />
    </div>

    <div class="card">
      <h3 style="margin-bottom:12px">📝 Event Log</h3>
      \${loading ? html\`<div style="text-align:center;padding:20px;color:var(--text2)">Loading...</div>\` : events.length === 0
        ? html\`<div style="text-align:center;padding:40px;color:var(--text2)">
            <div style="font-size:48px;margin-bottom:12px">🌟</div>
            <p>No activity yet. Start a conversation or run a scheduled task!</p>
          </div>\`
        : html\`<div style="display:flex;flex-direction:column;gap:8px">
            \${events.map(ev => html\`
              <div key=\${ev.timestamp} style="display:flex;align-items:center;gap:12px;padding:10px 14px;background:var(--bg2);border-radius:8px;border:1px solid var(--border)">
                <div style="font-size:20px">\${typeIcon(ev.event_type)}</div>
                <div style="flex:1">
                  <div style="display:flex;align-items:center;gap:8px">
                    <span class="badge \${typeBadge(ev.event_type)}">\${ev.event_type}</span>
                    <span style="color:var(--text2);font-size:12px">\${ev.agent}</span>
                  </div>
                  <div style="font-size:13px;margin-top:4px">\${ev.detail}</div>
                </div>
                <div style="font-family:var(--mono);font-size:11px;color:var(--text2)">\${fmtTime(ev.timestamp)}</div>
              </div>
            \`)}
          </div>\`
      }
    </div>
  </div>`;
}


// ═══ MAIN APP ═══
export function App() {
  const [currentPage, setPage] = useState('dashboard');
  const [lang, setLang] = useState(localStorage.getItem('bizclaw_lang') || 'vi');
  const [wsStatus, setWsStatus] = useState('disconnected');
  const [config, setConfig] = useState({});
  const [toast, setToast] = useState(null);
  const [paired, setPaired] = useState(false);
  const [checkingPairing, setCheckingPairing] = useState(true);
  const wsRef = useRef(null);

  // Global navigate function
  window._navigate = setPage;

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

  // History API navigation
  useEffect(() => {
    const handlePop = () => {
      const path = location.pathname.replace(/^\//, '').replace(/\/$/, '');
      const validPages = PAGES.filter(p => !p.sep).map(p => p.id);
      setPage(validPages.includes(path) ? path : 'dashboard');
    };
    window.addEventListener('popstate', handlePop);
    // Route from initial URL
    handlePop();
    return () => window.removeEventListener('popstate', handlePop);
  }, []);

  const navigate = useCallback((pageId) => {
    setPage(pageId);
    const path = '/' + (pageId === 'dashboard' ? '' : pageId);
    if (location.pathname !== path) {
      history.pushState({ page: pageId }, '', path);
    }
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

  // Render current page
  const renderPage = () => {
    switch (currentPage) {
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
      case 'brain': return html`<${BrainPage} lang=${lang} />`;
      case 'configfile': return html`<${ConfigFilePage} lang=${lang} />`;
      case 'scheduler': return html`<${SchedulerPage} lang=${lang} />`;
      case 'traces': return html`<${TracesPage} lang=${lang} />`;
      case 'cost': return html`<${CostPage} lang=${lang} />`;
      case 'activity': return html`<${ActivityPage} lang=${lang} />`;
      default: return html`<div class="card" style="padding:40px;text-align:center"><div style="font-size:48px;margin-bottom:16px">📄</div><h2>${currentPage}</h2></div>`;
    }
  };

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
        <main class="main">
          ${renderPage()}
        </main>
      </div>
      <${Toast} ...${toast || {}} />
    <//>
  `;
}
