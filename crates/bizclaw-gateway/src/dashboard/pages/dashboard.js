// DashboardPage — Enhanced with monitoring stats, mini-charts, agent cards
const { h, html, useState, useEffect, useContext, useCallback, useRef, useMemo } = window;
import { t, authFetch, authHeaders, StatsCard } from '/static/dashboard/shared.js';

// ── Mini SVG Bar Chart (pure inline, no library) ──
function MiniChart({ data = [], height = 48, color = 'var(--accent)', label }) {
  if (!data.length || data.every(v => v === 0)) {
    return html`<div style="text-align:center;padding:8px 0;color:var(--text2);font-size:11px">—</div>`;
  }
  const max = Math.max(...data, 1);
  const barW = Math.floor(100 / data.length);
  return html`<div style="position:relative">
    ${label ? html`<div style="font-size:10px;color:var(--text2);margin-bottom:4px">${label}</div>` : null}
    <svg width="100%" height=${height} viewBox="0 0 ${data.length * 12} ${height}" preserveAspectRatio="none" style="display:block;border-radius:6px;overflow:hidden">
      ${data.map((v, i) => {
        const h2 = Math.max((v / max) * (height - 4), 2);
        const opacity = 0.4 + (v / max) * 0.6;
        return html`<rect key=${i} x=${i * 12 + 1} y=${height - h2} width="10" height=${h2} rx="2" fill=${color} opacity=${opacity}>
          <animate attributeName="height" from="0" to=${h2} dur="0.4s" fill="freeze" begin="${i * 0.03}s"/>
          <animate attributeName="y" from=${height} to=${height - h2} dur="0.4s" fill="freeze" begin="${i * 0.03}s"/>
        </rect>`;
      })}
    </svg>
  </div>`;
}

// ── Status Dot with pulse animation ──
function StatusDot({ status = 'offline' }) {
  const colors = { online: '#22c55e', degraded: '#f59e0b', offline: '#ef4444', error: '#ef4444' };
  const c = colors[status] || colors.offline;
  return html`<span style="display:inline-block;width:8px;height:8px;border-radius:50%;background:${c};box-shadow:0 0 6px ${c};animation:${status === 'online' ? 'pulse 2s infinite' : 'none'}"></span>`;
}

// ── Agent Card with live stats ──
function AgentCard({ agent, lang }) {
  const statusLabel = { online: t('dash.online', lang), degraded: t('dash.degraded', lang) || 'Degraded', offline: t('dash.offline', lang) || 'Offline' };
  const status = agent.status || 'online';

  return html`<div class="card" style="position:relative;overflow:hidden;transition:transform 0.2s,box-shadow 0.2s"
    onMouseEnter=${e => { e.currentTarget.style.transform = 'translateY(-2px)'; e.currentTarget.style.boxShadow = '0 8px 24px rgba(0,0,0,0.2)'; }}
    onMouseLeave=${e => { e.currentTarget.style.transform = ''; e.currentTarget.style.boxShadow = ''; }}>
    <div style="display:flex;align-items:center;gap:10px;margin-bottom:12px">
      <div style="width:36px;height:36px;border-radius:10px;background:var(--grad1);display:flex;align-items:center;justify-content:center;font-size:18px">${agent.emoji || '🤖'}</div>
      <div style="flex:1;min-width:0">
        <div style="font-weight:600;font-size:14px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis">${agent.name}</div>
        <div style="display:flex;align-items:center;gap:6px;font-size:11px;color:var(--text2)">
          <${StatusDot} status=${status} />
          <span>${statusLabel[status] || status}</span>
        </div>
      </div>
    </div>
    <div style="display:grid;grid-template-columns:1fr 1fr;gap:8px;font-size:12px">
      <div>
        <div style="color:var(--text2);font-size:10px;text-transform:uppercase;letter-spacing:0.5px">${t('dash.model', lang)}</div>
        <div style="font-weight:500;margin-top:2px;color:var(--accent2);font-family:var(--mono);font-size:11px">${agent.model || '—'}</div>
      </div>
      <div>
        <div style="color:var(--text2);font-size:10px;text-transform:uppercase;letter-spacing:0.5px">${t('dash.channels', lang)}</div>
        <div style="font-weight:500;margin-top:2px">${agent.channels?.length || 0} ${t('th.channel', lang)}</div>
      </div>
    </div>
    ${agent.weeklyTokens?.length ? html`
      <div style="margin-top:10px;padding-top:10px;border-top:1px solid var(--border)">
        <${MiniChart} data=${agent.weeklyTokens} height=${32} color="var(--accent2)" label="${t('dash.token_7d', lang) || 'Token 7 ngày'}" />
      </div>
    ` : null}
  </div>`;
}

// ── Alert Banner ──
function AlertBanner({ alerts = [], lang }) {
  if (!alerts.length) return null;
  return html`<div style="margin-bottom:16px;display:flex;flex-direction:column;gap:6px">
    ${alerts.slice(0, 3).map((a, i) => html`
      <div key=${i} style="display:flex;align-items:center;gap:10px;padding:10px 14px;border-radius:10px;
        background:${a.level === 'critical' ? 'rgba(239,68,68,0.1)' : a.level === 'warning' ? 'rgba(245,158,11,0.1)' : 'rgba(59,130,246,0.1)'};
        border:1px solid ${a.level === 'critical' ? 'rgba(239,68,68,0.3)' : a.level === 'warning' ? 'rgba(245,158,11,0.3)' : 'rgba(59,130,246,0.3)'};
        animation:slideIn 0.3s ease ${i * 0.1}s both">
        <span style="font-size:18px">${a.level === 'critical' ? '🔴' : a.level === 'warning' ? '🟡' : '🔵'}</span>
        <div style="flex:1;font-size:13px">${a.message}</div>
        <span style="font-size:11px;color:var(--text2);font-family:var(--mono)">${a.time || ''}</span>
      </div>
    `)}
  </div>`;
}

function DashboardPage({ config, lang }) {
  const [clock, setClock] = useState('--:--:--');
  const [dateStr, setDateStr] = useState('');
  const [sysInfo, setSysInfo] = useState({});
  const [agents, setAgents] = useState([]);
  const [traces, setTraces] = useState([]);
  const [usage, setUsage] = useState({});
  const [alerts, setAlerts] = useState([]);
  const [activity, setActivity] = useState([]);

  // Clock
  useEffect(() => {
    const timer = setInterval(() => {
      const now = new Date();
      setClock(now.toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' }));
      setDateStr(now.toLocaleDateString(lang === 'vi' ? 'vi-VN' : 'en-US', { weekday: 'short', month: 'short', day: 'numeric' }));
    }, 1000);
    return () => clearInterval(timer);
  }, [lang]);

  // Fetch all monitoring data
  useEffect(() => {
    const fetchAll = async () => {
      try {
        const [infoRes, agentsRes, tracesRes, usageRes, actRes] = await Promise.allSettled([
          authFetch('/api/v1/info'),
          authFetch('/api/v1/agents'),
          authFetch('/api/v1/traces'),
          authFetch('/api/v1/usage'),
          authFetch('/api/v1/activity'),
        ]);
        if (infoRes.status === 'fulfilled') setSysInfo(await infoRes.value.json());
        if (agentsRes.status === 'fulfilled') {
          const d = await agentsRes.value.json();
          setAgents(d.agents || []);
        }
        if (tracesRes.status === 'fulfilled') {
          const d = await tracesRes.value.json();
          setTraces(d.traces || []);
        }
        if (usageRes.status === 'fulfilled') setUsage(await usageRes.value.json());
        if (actRes.status === 'fulfilled') {
          const d = await actRes.value.json();
          setActivity((d.events || []).slice(0, 8));
        }
      } catch (e) { console.warn('Dashboard fetch:', e); }
    };
    fetchAll();
    const timer = setInterval(fetchAll, 15000);
    return () => clearInterval(timer);
  }, []);

  // Generate alerts from monitoring data
  useEffect(() => {
    const newAlerts = [];
    const totalTokens = traces.reduce((s, t) => s + (t.total_tokens || 0), 0);
    if (totalTokens > 100000) {
      newAlerts.push({ level: 'warning', message: t('alert.high_token', lang) || `Token sử dụng cao: ${(totalTokens/1000).toFixed(1)}k tokens hôm nay`, time: 'Hôm nay' });
    }
    const slowTraces = traces.filter(t => (t.duration_ms || 0) > 5000);
    if (slowTraces.length > 3) {
      newAlerts.push({ level: 'warning', message: t('alert.slow_response', lang) || `${slowTraces.length} request phản hồi chậm (>5s)`, time: 'Gần đây' });
    }
    const errorTraces = traces.filter(t => t.error);
    if (errorTraces.length > 0) {
      newAlerts.push({ level: 'critical', message: t('alert.errors', lang) || `${errorTraces.length} lỗi API gần đây`, time: 'Gần đây' });
    }
    setAlerts(newAlerts);
  }, [traces, lang]);

  const provider = sysInfo.default_provider || config?.default_provider || '—';
  const model = config?.default_model || sysInfo.default_model || '—';
  const version = sysInfo.version || config?.version || '—';
  const uptimeSecs = sysInfo.uptime_secs || 0;
  const uptimeStr = uptimeSecs > 0
    ? (uptimeSecs >= 86400 ? Math.floor(uptimeSecs/86400) + 'd ' : '')
      + (uptimeSecs >= 3600 ? Math.floor((uptimeSecs%86400)/3600) + 'h ' : '')
      + Math.floor((uptimeSecs%3600)/60) + 'm'
    : '—';
  const [osName, archName] = (sysInfo.platform || '').split('/');

  // Compute stats from traces
  const totalTokens = traces.reduce((s, t) => s + (t.total_tokens || 0), 0);
  const avgResponseMs = traces.length > 0 ? Math.round(traces.reduce((s, t) => s + (t.duration_ms || 0), 0) / traces.length) : 0;
  const totalCost = traces.reduce((s, t) => s + (t.cost_usd || 0), 0);

  // Weekly data (group traces by day for chart)
  const weeklyData = useMemo(() => {
    const days = Array(7).fill(0);
    const now = Date.now();
    traces.forEach(t => {
      const age = now - new Date(t.timestamp || 0).getTime();
      const dayIdx = 6 - Math.floor(age / 86400000);
      if (dayIdx >= 0 && dayIdx < 7) days[dayIdx] += (t.total_tokens || 0);
    });
    return days;
  }, [traces]);

  const responseData = useMemo(() => {
    const days = Array(7).fill(0);
    const counts = Array(7).fill(0);
    const now = Date.now();
    traces.forEach(t => {
      const age = now - new Date(t.timestamp || 0).getTime();
      const dayIdx = 6 - Math.floor(age / 86400000);
      if (dayIdx >= 0 && dayIdx < 7) { days[dayIdx] += (t.duration_ms || 0); counts[dayIdx]++; }
    });
    return days.map((d, i) => counts[i] > 0 ? Math.round(d / counts[i]) : 0);
  }, [traces]);

  const [showWelcome, setShowWelcome] = useState(() => !localStorage.getItem('bizclaw_welcome_dismissed'));
  const dismissWelcome = () => { localStorage.setItem('bizclaw_welcome_dismissed', '1'); setShowWelcome(false); };

  return html`<div>
    <div class="page-header"><div>
      <h1>⚡ MAMA Tổng Quản</h1>
      <div class="sub">${t('dash.subtitle', lang)}</div>
    </div></div>

    ${/* ── Welcome Banner (first visit) ── */null}
    ${showWelcome && html`
      <div style="margin-bottom:16px;padding:20px 24px;border-radius:12px;background:linear-gradient(135deg,rgba(59,130,246,0.12),rgba(168,85,247,0.08));border:1px solid rgba(59,130,246,0.2);position:relative;animation:fadeIn 0.5s">
        <button onClick=${dismissWelcome} style="position:absolute;top:10px;right:14px;background:none;border:none;color:var(--text2);font-size:18px;cursor:pointer;padding:4px;line-height:1" title="Đóng">✕</button>
        <div style="display:flex;align-items:center;gap:16px;flex-wrap:wrap">
          <div style="font-size:40px">👋</div>
          <div style="flex:1;min-width:200px">
            <div style="font-size:16px;font-weight:700;color:var(--text);margin-bottom:4px">${lang === 'vi' ? 'Chào mừng đến BizClaw!' : 'Welcome to BizClaw!'}</div>
            <div style="font-size:13px;color:var(--text2);line-height:1.5">${lang === 'vi' 
              ? 'Bắt đầu bằng cách tạo Agent AI, kết nối kênh chat (Zalo/Telegram), và nạp tri thức cho Bot. Chỉ cần 5 phút!'
              : 'Start by creating an AI Agent, connecting chat channels, and uploading knowledge. Just 5 minutes!'}</div>
          </div>
          <div style="display:flex;gap:8px;flex-wrap:wrap">
            <button class="btn btn-primary btn-sm" onClick=${() => window._navigate && window._navigate('wiki')} style="white-space:nowrap">
              🚀 ${lang === 'vi' ? 'Quick Start Guide' : 'Quick Start Guide'}
            </button>
            <button class="btn btn-outline btn-sm" onClick=${() => window._navigate && window._navigate('agents')} style="white-space:nowrap">
              🤖 ${lang === 'vi' ? 'Tạo Agent' : 'Create Agent'}
            </button>
          </div>
        </div>
      </div>
    `}

    <!-- Alerts -->
    <${AlertBanner} alerts=${alerts} lang=${lang} />

    <!-- Top Stats Row -->
    <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(140px,1fr));gap:12px;margin-bottom:16px">
      <${StatsCard} label=${t('dash.clock', lang)} value=${clock} color="accent" sub=${dateStr} icon="⏰" />
      <${StatsCard} label=${t('dash.uptime', lang)} value=${uptimeStr} color="green" icon="🟢" />
      <${StatsCard} label="Agents" value=${agents.length} color="blue" icon="🤖" sub="${agents.filter(a => a.status !== 'offline').length} ${t('dash.online', lang)}" />
      <${StatsCard} label="Tokens" value=${(totalTokens > 1000 ? (totalTokens/1000).toFixed(1) + 'k' : totalTokens)} color="accent2" icon="🎯" sub=${t('dash.today', lang) || 'Hôm nay'} />
      <${StatsCard} label=${t('dash.avg_response', lang) || 'Phản hồi TB'} value=${avgResponseMs + 'ms'} color=${avgResponseMs > 3000 ? 'red' : 'green'} icon="⚡" />
      <${StatsCard} label=${t('dash.cost', lang) || 'Chi phí'} value=${'$' + totalCost.toFixed(3)} color="accent" icon="💰" />
    </div>

    <!-- Charts Row -->
    <div style="display:grid;grid-template-columns:1fr 1fr;gap:14px;margin-bottom:16px">
      <div class="card">
        <div class="card-label" style="margin-bottom:8px">📈 ${t('dash.token_trend', lang) || 'Token 7 ngày qua'}</div>
        <${MiniChart} data=${weeklyData} height=${64} color="var(--accent2)" />
        <div style="display:flex;justify-content:space-between;font-size:10px;color:var(--text2);margin-top:4px">
          <span>7d ago</span><span>Today</span>
        </div>
      </div>
      <div class="card">
        <div class="card-label" style="margin-bottom:8px">⚡ ${t('dash.response_trend', lang) || 'Response Time 7 ngày'}</div>
        <${MiniChart} data=${responseData} height=${64} color="var(--green)" />
        <div style="display:flex;justify-content:space-between;font-size:10px;color:var(--text2);margin-top:4px">
          <span>7d ago</span><span>Today</span>
        </div>
      </div>
    </div>

    <!-- System + Quick Actions -->
    <div style="display:grid;grid-template-columns:1fr 1fr;gap:14px;margin-bottom:16px">
      <div class="card">
        <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:12px">
          <div class="card-label" style="margin:0">🖥️ ${t('dash.system', lang)}</div>
          <span class="badge badge-green">● ${t('dash.online', lang)}</span>
        </div>
        <div style="display:grid;grid-template-columns:1fr 1fr;gap:8px;font-size:12px">
          <div><span style="color:var(--text2)">${t('sys.os', lang)}</span> ${osName || '—'}</div>
          <div><span style="color:var(--text2)">${t('sys.arch', lang)}</span> ${archName || '—'}</div>
          <div><span style="color:var(--text2)">SIMD:</span> <span style="color:var(--accent2)">${archName === 'aarch64' ? 'NEON' : archName === 'x86_64' ? 'AVX2' : '—'}</span></div>
          <div><span style="color:var(--text2)">${t('sys.memory', lang)}</span> ${sysInfo.memory || '—'}</div>
          <div><span style="color:var(--text2)">${t('dash.provider', lang)}:</span> <b>${provider}</b></div>
          <div><span style="color:var(--text2)">${t('dash.model', lang)}:</span> <b style="color:var(--accent2)">${model}</b></div>
        </div>
      </div>
      <div class="card">
        <div class="card-label" style="margin-bottom:10px">⚡ ${t('dash.quickactions', lang)}</div>
        <div style="display:flex;flex-wrap:wrap;gap:6px">
          ${[
            { id: 'chat', icon: '💬', label: lang === 'vi' ? 'Trò chuyện' : 'Chat' },
            { id: 'agents', icon: '🤖', label: 'AI Agent' },
            { id: 'channels', icon: '📱', label: lang === 'vi' ? 'Kênh' : 'Channels' },
            { id: 'knowledge', icon: '📚', label: 'RAG' },
            { id: 'handoff', icon: '🤝', label: 'Handoff' },
            { id: 'campaigns', icon: '📢', label: 'Broadcast' },
          ].map(p => html`
            <button class="btn btn-outline btn-sm" key=${p.id}
              onClick=${() => window._navigate && window._navigate(p.id)}>
              ${p.icon} ${p.label}
            </button>
          `)}
        </div>
      </div>
    </div>

    <!-- Agent Cards -->
    ${agents.length > 0 ? html`
      <div style="margin-bottom:16px">
        <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:10px">
          <div class="card-label" style="margin:0;font-size:14px">🤖 ${t('agents.title', lang)} (${agents.length})</div>
          <button class="btn btn-outline btn-sm" onClick=${() => window._navigate && window._navigate('agents')}>
            ${t('dash.viewall', lang) || 'Xem tất cả'} →
          </button>
        </div>
        <div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(220px,1fr));gap:12px">
          ${agents.slice(0, 6).map(a => html`<${AgentCard} key=${a.name} agent=${a} lang=${lang} />`)}
        </div>
      </div>
    ` : null}

    <!-- Recent Activity -->
    ${activity.length > 0 ? html`
      <div class="card">
        <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:12px">
          <div class="card-label" style="margin:0">⚡ ${t('dash.recent_activity', lang) || 'Hoạt động gần đây'}</div>
          <button class="btn btn-outline btn-sm" onClick=${() => window._navigate && window._navigate('activity')}>
            ${t('dash.viewall', lang) || 'Xem tất cả'} →
          </button>
        </div>
        <div style="display:flex;flex-direction:column;gap:6px">
          ${activity.map((ev, i) => html`
            <div key=${i} style="display:flex;align-items:center;gap:10px;padding:8px 12px;background:var(--bg2);border-radius:8px;font-size:12px;
              animation:slideIn 0.3s ease ${i * 0.05}s both">
              <span style="font-size:16px">${ev.event_type?.includes('llm') ? '🤖' : ev.event_type?.includes('tool') ? '🛠️' : ev.event_type?.includes('scheduler') ? '⏰' : '⚡'}</span>
              <div style="flex:1;min-width:0">
                <span class="badge ${ev.event_type?.includes('error') ? 'badge-red' : ev.event_type?.includes('completed') ? 'badge-green' : 'badge-blue'}" style="font-size:10px">${String(ev.event_type).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'})[c])}</span>
                <span style="margin-left:6px;color:var(--text2)">${ev.agent || ''}</span>
              </div>
              <span style="color:var(--text2);font-family:var(--mono);font-size:10px">${new Date(ev.timestamp).toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit' })}</span>
            </div>
          `)}
        </div>
      </div>
    ` : null}
  </div>`;
}


export { DashboardPage };
