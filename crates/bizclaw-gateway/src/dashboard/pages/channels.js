// ChannelsPage — Multi-Instance Channel Management
// Supports adding multiple Telegram bots, Zalo accounts, etc.
// Uses /api/v1/channel-instances CRUD API
const { h, html, useState, useEffect, useContext, useCallback, useRef, useMemo } = window;
import { t, authFetch, authHeaders, StatsCard } from '/static/dashboard/shared.js';

function ChannelsPage({ lang }) {
  const { showToast } = useContext(AppContext);
  const [instances, setInstances] = useState([]);
  const [agents, setAgents] = useState([]);
  const [loading, setLoading] = useState(true);
  const [configCh, setConfigCh] = useState(null);
  const [chForm, setChForm] = useState({});
  const [zaloQr, setZaloQr] = useState(null);
  const [zaloLoading, setZaloLoading] = useState(false);
  const [showAddNew, setShowAddNew] = useState(false);
  const [newChType, setNewChType] = useState('');
  const [newChName, setNewChName] = useState('');
  const [deleteConfirm, setDeleteConfirm] = useState(null);
  const inp = 'width:100%;padding:8px;margin-top:4px;background:var(--bg2);border:1px solid var(--border);border-radius:6px;color:var(--text);font-size:13px';

  // Channel type definitions — which types exist and their config fields
  const channelDefs = [
    {name:'cli',icon:'💻',label:'CLI Terminal',type:'interactive',alwaysActive:true},
    {name:'telegram',icon:'📱',label:'Telegram Bot',type:'messaging',multi:true,
     fields:[{key:'bot_token',label:'Bot Token',secret:true},{key:'allowed_chat_ids',label:'Allowed Chat IDs',placeholder:'-100123, 987654'}]},
    {name:'zalo',icon:'💙',label:'Zalo Cá Nhân',type:'messaging',hasQr:true,multi:true,
     fields:[{key:'cookie',label:'Cookie (từ chat.zalo.me)',secret:true,textarea:true},{key:'imei',label:'IMEI (Device ID)',placeholder:'Tự tạo nếu để trống'}]},
    {name:'zalo_oa',icon:'🏪',label:'Zalo OA (Official Account)',type:'messaging',multi:true,
     fields:[{key:'oa_id',label:'OA ID',placeholder:'1234567890'},{key:'app_id',label:'App ID (từ developers.zalo.me)',placeholder:'4321...'},{key:'app_secret',label:'App Secret Key',secret:true},{key:'access_token',label:'OA Access Token',secret:true},{key:'refresh_token',label:'Refresh Token',secret:true},{key:'webhook_url',label:'Webhook URL (BizClaw tự tạo)',placeholder:'https://your-domain/api/v1/webhooks/zalo-oa'}]},
    {name:'zalo_bot',icon:'🤖',label:'Zalo Bot (Bot Platform)',type:'messaging',multi:true,
      fields:[{key:'bot_token',label:'Bot Token (từ Bot Creator Mini App)',secret:true,placeholder:'123456:abc123xyz'},{key:'secret_token',label:'Secret Token (webhook validation, 8-256 ký tự)',secret:true,placeholder:'mykey-abcyxz'},{key:'webhook_url',label:'Webhook URL (để trống = dùng Polling)',placeholder:'https://your-domain/api/v1/webhook/zalo-bot'}]},
    {name:'messenger',icon:'💜',label:'Facebook Messenger',type:'messaging',multi:true,
     fields:[{key:'page_id',label:'Page ID',placeholder:'1234567890'},{key:'page_access_token',label:'Page Access Token',secret:true},{key:'app_secret',label:'App Secret (verify signature)',secret:true},{key:'verify_token',label:'Webhook Verify Token',placeholder:'bizclaw_verify_2024'},{key:'webhook_url',label:'Webhook URL',placeholder:'https://your-domain/api/v1/webhooks/messenger'}]},
    {name:'discord',icon:'🎮',label:'Discord Bot',type:'messaging',multi:true,
     fields:[{key:'bot_token',label:'Bot Token',secret:true},{key:'allowed_channel_ids',label:'Allowed Channel IDs',placeholder:'123456, 789012'}]},
    {name:'email',icon:'📧',label:'Email (IMAP/SMTP)',type:'messaging',multi:true,
     fields:[{key:'smtp_host',label:'SMTP Host',placeholder:'smtp.gmail.com'},{key:'smtp_port',label:'SMTP Port',placeholder:'587'},
             {key:'smtp_user',label:'Email Address',placeholder:'bot@example.com'},{key:'smtp_pass',label:'App Password',secret:true},
             {key:'imap_host',label:'IMAP Host',placeholder:'imap.gmail.com'}]},
    {name:'whatsapp',icon:'💬',label:'WhatsApp Business',type:'messaging',multi:true,
     fields:[{key:'phone_number_id',label:'Phone Number ID'},{key:'access_token',label:'Access Token',secret:true},{key:'business_id',label:'Business ID'}]},
    {name:'webhook',icon:'🌐',label:'Webhook',type:'api',multi:true,
     fields:[{key:'webhook_url',label:'Outbound URL',placeholder:'https://example.com/webhook'},{key:'webhook_secret',label:'Secret',secret:true}]},
  ];

  // ── Load data ──
  const [zaloStatus, setZaloStatus] = useState(null);

  const load = async () => {
    try {
      const [instRes, agentRes] = await Promise.all([
        authFetch('/api/v1/channel-instances'),
        authFetch('/api/v1/agents'),
      ]);
      const instData = await instRes.json();
      const agentData = await agentRes.json();
      setInstances(instData.instances || []);
      setAgents(agentData.agents || []);
    } catch(e) {
      console.error('Channels load:', e);
      setInstances([]);
      setAgents([]);
    }
    setLoading(false);
  };

  // Poll for Zalo native session status
  useEffect(() => {
    const fetchZaloStatus = async () => {
      try {
        const r = await authFetch('/api/v1/channels/zalo/session');
        const data = await r.json();
        setZaloStatus(data);
      } catch(e) {}
    };
    fetchZaloStatus();
    const interval = setInterval(fetchZaloStatus, 15000); // Check every 15s
    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    const timeout = setTimeout(() => setLoading(false), 8000);
    load().finally(() => clearTimeout(timeout));
    return () => clearTimeout(timeout);
  }, []);

  // ── Build display list: instances from API + unconfigured types ──
  const getDisplayList = () => {
    const list = [];
    // CLI always first
    list.push({ id: 'cli', name: 'CLI Terminal', channel_type: 'cli', enabled: true, agent_name: '', config: {}, icon: '💻', status: 'active' });
    // Instance list from API
    for (const inst of instances) {
      const def = channelDefs.find(d => d.name === inst.channel_type);
      list.push({
        ...inst,
        icon: def?.icon || '📡',
        status: inst.enabled ? 'active' : 'configured',
      });
    }
    // Unconfigured channel types (shown as "available")
    for (const def of channelDefs) {
      if (def.name === 'cli') continue;
      if (!instances.find(i => i.channel_type === def.name)) {
        list.push({
          id: 'avail_' + def.name,
          name: def.label,
          channel_type: def.name,
          enabled: false,
          agent_name: '',
          config: {},
          icon: def.icon,
          status: 'available',
        });
      }
    }
    return list;
  };

  // ── Open config form ──
  const openConfig = (inst) => {
    const def = channelDefs.find(d => d.name === inst.channel_type);
    if (!def || !def.fields) return;
    setConfigCh({ ...def, instanceId: inst.id, instanceName: inst.name || def.label });
    setZaloQr(null);
    // Pre-fill form
    const cfg = inst.config || {};
    const f = {
      enabled: inst.enabled !== false,
      display_name: inst.name || '',
      agent_name: inst.agent_name || '',
    };
    (def.fields || []).forEach(fd => { f[fd.key] = cfg[fd.key] || ''; });
    setChForm(f);
  };

  // ── Save (create or update) via instance API ──
  const saveChannelConfig = async () => {
    if (!configCh) return;
    try {
      const config = {};
      (configCh.fields || []).forEach(fd => {
        if (chForm[fd.key]) config[fd.key] = chForm[fd.key];
      });

      const isNew = !configCh.instanceId || configCh.instanceId.startsWith('avail_');
      const body = {
        ...(isNew ? {} : { id: configCh.instanceId }),
        name: chForm.display_name || configCh.instanceName || configCh.name,
        channel_type: configCh.name,
        enabled: chForm.enabled !== false,
        agent_name: chForm.agent_name || '',
        config,
      };

      const r = await authFetch('/api/v1/channel-instances', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      const d = await r.json();
      if (d.ok) {
        showToast('✅ Đã lưu ' + (chForm.display_name || configCh.name), 'success');
        setConfigCh(null);
        load();
      } else {
        showToast('❌ ' + (d.error || 'Lỗi lưu kênh'), 'error');
      }
    } catch(e) {
      showToast('❌ ' + e.message, 'error');
    }
  };

  // ── Delete instance ──
  const deleteInstance = async (id) => {
    try {
      const r = await authFetch('/api/v1/channel-instances/' + encodeURIComponent(id), { method: 'DELETE' });
      const d = await r.json();
      if (d.ok) {
        showToast('🗑️ Đã xoá kênh', 'success');
        setDeleteConfirm(null);
        load();
      } else {
        showToast('❌ ' + (d.error || 'Lỗi xoá'), 'error');
      }
    } catch(e) {
      showToast('❌ ' + e.message, 'error');
    }
  };

  // ── Add new channel instance ──
  const addNewChannel = () => {
    if (!newChType) { showToast('⚠️ Chọn loại kênh', 'error'); return; }
    const def = channelDefs.find(d => d.name === newChType);
    if (!def) return;
    const displayName = newChName.trim() || (def.label + ' #' + (instances.filter(i => i.channel_type === newChType).length + 1));
    setConfigCh({ ...def, instanceId: null, instanceName: displayName });
    const f = { enabled: true, display_name: displayName, agent_name: '' };
    (def.fields || []).forEach(fd => { f[fd.key] = ''; });
    setChForm(f);
    setShowAddNew(false);
    setNewChType('');
    setNewChName('');
  };

  // ── Zalo QR ──
  const loadZaloQr = async () => {
    setZaloLoading(true);
    try {
      const r = await authFetch('/api/v1/zalo/qr', { method: 'POST' });
      const d = await r.json();
      if (d.ok) { setZaloQr(d); if (d.imei) setChForm(f => ({ ...f, imei: d.imei })); }
      else showToast('❌ ' + (d.error || 'Không thể tạo QR'), 'error');
    } catch(e) { showToast('❌ ' + e.message, 'error'); }
    setZaloLoading(false);
  };

  // ── Color map for channel types ──
  const typeColors = {
    cli: { bg: 'rgba(139,92,246,0.1)', border: 'rgba(139,92,246,0.3)', glow: 'rgba(139,92,246,0.15)' },
    telegram: { bg: 'rgba(0,136,204,0.1)', border: 'rgba(0,136,204,0.3)', glow: 'rgba(0,136,204,0.15)' },
    zalo: { bg: 'rgba(0,106,255,0.1)', border: 'rgba(0,106,255,0.3)', glow: 'rgba(0,106,255,0.15)' },
    zalo_oa: { bg: 'rgba(0,106,255,0.1)', border: 'rgba(0,148,255,0.4)', glow: 'rgba(0,148,255,0.2)' },
    zalo_bot: { bg: 'rgba(0,180,80,0.1)', border: 'rgba(0,180,80,0.35)', glow: 'rgba(0,180,80,0.2)' },
    messenger: { bg: 'rgba(160,70,255,0.1)', border: 'rgba(160,70,255,0.3)', glow: 'rgba(160,70,255,0.15)' },
    discord: { bg: 'rgba(88,101,242,0.1)', border: 'rgba(88,101,242,0.3)', glow: 'rgba(88,101,242,0.15)' },
    email: { bg: 'rgba(234,179,8,0.1)', border: 'rgba(234,179,8,0.3)', glow: 'rgba(234,179,8,0.15)' },
    whatsapp: { bg: 'rgba(37,211,102,0.1)', border: 'rgba(37,211,102,0.3)', glow: 'rgba(37,211,102,0.15)' },
    webhook: { bg: 'rgba(249,115,22,0.1)', border: 'rgba(249,115,22,0.3)', glow: 'rgba(249,115,22,0.15)' },
  };

  if (loading) return html`<div class="card" style="text-align:center;padding:40px;color:var(--text2)">Đang tải kênh liên lạc...</div>`;

  const displayList = getDisplayList();
  const activeCount = displayList.filter(i => i.status === 'active').length;
  const totalInstances = instances.length;
  const multiCapable = channelDefs.filter(d => d.multi);

  return html`<div>
    <div class="page-header"><div><h1>📡 ${t('channels.title', lang)}</h1><div class="sub">${t('channels.subtitle', lang)} — Hỗ trợ nhiều kênh cùng loại (multi-instance)</div></div>
      <button class="btn" style="background:var(--grad1);color:#fff;padding:8px 18px" onClick=${() => setShowAddNew(!showAddNew)}>+ Thêm kênh mới</button>
    </div>
    <div class="stats">
      <${StatsCard} label="Tổng instance" value=${totalInstances} color="accent" icon="📡" />
      <${StatsCard} label="Đang hoạt động" value=${activeCount} color="green" icon="✅" />
      <${StatsCard} label="Loại kênh" value=${channelDefs.length} color="blue" icon="🔌" />
    </div>

    ${showAddNew && html`
      <div class="card" style="margin-bottom:14px;border:1px solid var(--accent)">
        <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px">
          <h3 style="margin:0">➕ Thêm kênh liên lạc mới</h3>
          <button class="btn btn-outline btn-sm" onClick=${() => setShowAddNew(false)}>✕</button>
        </div>
        <div style="display:grid;grid-template-columns:1fr 1fr;gap:10px;font-size:13px">
          <label>Loại kênh
            <select style="${inp};cursor:pointer" value=${newChType} onChange=${e => setNewChType(e.target.value)}>
              <option value="">— Chọn loại kênh —</option>
              ${multiCapable.map(d => html`<option key=${d.name} value=${d.name}>${d.icon} ${d.label}</option>`)}
            </select>
          </label>
          <label>Tên hiển thị
            <input style="${inp}" value=${newChName} onInput=${e => setNewChName(e.target.value)} placeholder="VD: Bot bán hàng, Support Bot 2..." />
          </label>
        </div>
        ${newChType && html`
          <div style="margin-top:10px;padding:10px;background:var(--bg2);border-radius:8px;font-size:12px;color:var(--text2)">
            💡 <strong>${channelDefs.find(d => d.name === newChType)?.icon} ${channelDefs.find(d => d.name === newChType)?.label}</strong> —
            Bạn có thể tạo nhiều instance cùng loại. Mỗi instance có token/config riêng và gán cho agent khác nhau.
            <br/>Hiện có: <strong>${instances.filter(i => i.channel_type === newChType).length}</strong> instance loại này.
          </div>
        `}
        <div style="margin-top:12px;display:flex;gap:8px;justify-content:flex-end">
          <button class="btn btn-outline" onClick=${() => setShowAddNew(false)}>Huỷ</button>
          <button class="btn" style="background:var(--grad1);color:#fff;padding:8px 20px" onClick=${addNewChannel}>➕ Tạo kênh</button>
        </div>
      </div>
    `}

    ${/* 🚨 ZALO HEALTH WARNING BANNER 🚨 */null}
    ${zaloStatus && zaloStatus.online === false && html`
      <div class="card" style="margin-bottom:14px; background: rgba(239, 68, 68, 0.1); border: 2px solid var(--red); color: var(--text);">
        <div style="display:flex; align-items:center; gap: 16px;">
          <div style="font-size: 32px; animation: pulse 2s infinite;">⚠️</div>
          <div style="flex: 1;">
            <h3 style="margin: 0 0 4px 0; color: #ff6b6b;">Kết nối Zalo Đã Bị Mất!</h3>
            <p style="margin: 0; font-size: 13px; color: var(--text2);">
              Tài khoản Zalo AI của bạn đã bị văng ra (do điện thoại đăng nhập đè hoặc hết hạn cookie). 
              <strong>Trợ lý AI sẽ ngừng rep tin nhắn cho đến khi bạn quét lại QR.</strong>
            </p>
          </div>
          <button class="btn" style="background: var(--red); color: #fff; padding: 10px 20px; font-weight: bold;"
            onClick=${() => {
              const zaloInst = instances.find(i => i.channel_type === 'zalo');
              if (zaloInst) openConfig(zaloInst);
            }}
          >
            🔲 Quét Mã QR Ngay
          </button>
        </div>
      </div>
    `}

    ${configCh && html`
      <div class="card" style="margin-bottom:14px;border:1px solid var(--accent)">
        <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:14px">
          <h3 style="margin:0">${configCh.icon} Cấu hình: ${chForm.display_name || configCh.instanceName}</h3>
          <button class="btn btn-outline btn-sm" onClick=${() => setConfigCh(null)}>✕ Đóng</button>
        </div>

        <div style="display:grid;grid-template-columns:1fr 1fr;gap:10px;margin-bottom:14px">
          ${''}
          <div style="display:flex;align-items:center;gap:8px;padding:10px;background:var(--bg2);border-radius:8px">
            <span style="font-size:13px">Kích hoạt:</span>
            <div style="position:relative;width:44px;height:24px;background:${chForm.enabled ? 'var(--green)' : 'var(--bg3)'};border-radius:12px;cursor:pointer;transition:background 0.3s" onClick=${() => setChForm(f => ({ ...f, enabled: !f.enabled }))}>
              <div style="position:absolute;top:2px;left:${chForm.enabled ? '22px' : '2px'};width:20px;height:20px;background:#fff;border-radius:50%;transition:left 0.3s;box-shadow:0 1px 3px rgba(0,0,0,0.3)"></div>
            </div>
            <span style="font-size:12px;color:${chForm.enabled ? 'var(--green)' : 'var(--text2)'}">${chForm.enabled ? 'Bật' : 'Tắt'}</span>
          </div>

          ${''}
          <label style="font-size:13px">Gán Agent
            <select style="${inp};cursor:pointer" value=${chForm.agent_name || ''} onChange=${e => setChForm(f => ({ ...f, agent_name: e.target.value }))}>
              <option value="">— Không gán (dùng default) —</option>
              ${agents.map(ag => html`<option key=${ag.name} value=${ag.name}>🤖 ${ag.name}${ag.role ? ` (${ag.role})` : ''}</option>`)}
            </select>
          </label>
        </div>

        <div style="margin-bottom:10px">
          <label style="font-size:13px">Tên hiển thị
            <input style="${inp}" value=${chForm.display_name || ''} onInput=${e => setChForm(f => ({ ...f, display_name: e.target.value }))} placeholder="Tên tuỳ chỉnh cho kênh này" />
          </label>
        </div>

        ${configCh.hasQr && html`
          <div style="margin-bottom:14px;padding:12px;background:var(--bg2);border-radius:8px;border:1px solid var(--border)">
            <div style="display:flex;align-items:center;gap:8px;margin-bottom:8px">
              <strong style="font-size:13px">📱 Đăng nhập Zalo bằng QR</strong>
              <button class="btn" style="background:var(--grad1);color:#fff;padding:4px 12px;font-size:12px" onClick=${loadZaloQr} disabled=${zaloLoading}>${zaloLoading ? 'Đang tạo...' : '🔲 Quét QR'}</button>
            </div>
            ${zaloQr && html`
              <div style="text-align:center;padding:10px">
                ${zaloQr.qr_code ? html`<img src="${zaloQr.qr_code.startsWith('data:') ? zaloQr.qr_code : 'data:image/png;base64,' + zaloQr.qr_code}" style="width:200px;height:200px;border-radius:8px;border:2px solid var(--accent)" />` : html`<div style="color:var(--text2)">Không thể hiển thị QR</div>`}
                <div style="font-size:12px;color:var(--text2);margin-top:8px">${zaloQr.message || 'Quét mã QR bằng Zalo trên điện thoại'}</div>
              </div>
            `}
            <div style="font-size:11px;color:var(--text2);margin-top:6px">Hoặc paste cookie từ chat.zalo.me vào ô bên dưới</div>
          </div>
        `}

        <div style="display:grid;gap:10px;font-size:13px">
          ${(configCh.fields || []).map(fd => html`
            <label key=${fd.key}>${fd.label}
              ${fd.textarea ? html`<textarea style="${inp};min-height:80px;font-family:var(--mono);resize:vertical" value=${chForm[fd.key] || ''} onInput=${e => setChForm(f => ({ ...f, [fd.key]: e.target.value }))} placeholder=${fd.placeholder || 'Nhập ' + fd.label + '...'} />` :
              html`<input style="${inp}" type=${fd.secret ? 'password' : 'text'} value=${chForm[fd.key] || ''} onInput=${e => setChForm(f => ({ ...f, [fd.key]: e.target.value }))} placeholder=${fd.placeholder || 'Nhập ' + fd.label + '...'} />`}
            </label>
          `)}
        </div>
        <div style="margin-top:14px;display:flex;gap:8px;justify-content:flex-end">
          <button class="btn btn-outline" onClick=${() => setConfigCh(null)}>Huỷ</button>
          <button class="btn" style="background:var(--grad1);color:#fff;padding:8px 20px" onClick=${saveChannelConfig}>💾 Lưu cấu hình</button>
        </div>
      </div>
    `}

    ${deleteConfirm && html`
      <div style="position:fixed;inset:0;background:rgba(0,0,0,0.5);z-index:200;display:flex;align-items:center;justify-content:center" onClick=${() => setDeleteConfirm(null)}>
        <div style="background:var(--surface);padding:24px;border-radius:12px;width:360px;text-align:center" onClick=${e => e.stopPropagation()}>
          <div style="font-size:28px;margin-bottom:8px">🗑️</div>
          <h3>Xoá kênh này?</h3>
          <p style="color:var(--text2);font-size:13px;margin-bottom:16px">Bạn có chắc muốn xoá <strong>${deleteConfirm.name}</strong>? Thao tác không thể hoàn tác.</p>
          <div style="display:flex;gap:8px;justify-content:center">
            <button class="btn btn-outline" onClick=${() => setDeleteConfirm(null)}>Huỷ</button>
            <button class="btn" style="background:var(--red);color:#fff;padding:8px 20px" onClick=${() => deleteInstance(deleteConfirm.id)}>🗑️ Xoá</button>
          </div>
        </div>
      </div>
    `}

    <div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(260px,1fr));gap:14px">
      ${displayList.map(inst => {
        const def = channelDefs.find(d => d.name === inst.channel_type);
        const colors = typeColors[inst.channel_type] || typeColors.webhook;
        const isAvailable = inst.status === 'available';
        const isActive = inst.status === 'active';

        return html`
          <div key=${inst.id}
            style="position:relative;padding:20px;border-radius:14px;
              background:${isAvailable ? 'transparent' : colors.bg};
              border:${isAvailable ? '2px dashed var(--border)' : `1px solid ${colors.border}`};
              ${isActive ? `box-shadow:0 0 20px ${colors.glow},0 4px 15px rgba(0,0,0,0.1)` : ''};
              transition:all 0.3s ease;cursor:pointer;
              opacity:${isAvailable ? '0.6' : '1'}"
            onClick=${() => { if (isAvailable && def?.multi) { setNewChType(inst.channel_type); addNewChannel(); } else if (def?.fields) openConfig(inst); }}
            onMouseOver=${e => { e.currentTarget.style.transform = 'translateY(-2px)'; e.currentTarget.style.boxShadow = `0 0 25px ${colors.glow},0 8px 25px rgba(0,0,0,0.15)`; }}
            onMouseOut=${e => { e.currentTarget.style.transform = ''; e.currentTarget.style.boxShadow = isActive ? `0 0 20px ${colors.glow},0 4px 15px rgba(0,0,0,0.1)` : ''; }}>

            ${isActive && html`<div style="position:absolute;top:12px;right:14px;width:8px;height:8px;border-radius:50%;background:var(--green);box-shadow:0 0 6px var(--green);animation:pulse 2s infinite"></div>`}
            ${!isActive && !isAvailable && html`<div style="position:absolute;top:12px;right:14px;width:8px;height:8px;border-radius:50%;background:var(--text2);opacity:0.4"></div>`}

            <div style="display:flex;align-items:center;gap:12px;margin-bottom:12px">
              <div style="width:48px;height:48px;border-radius:12px;background:${colors.bg};border:1px solid ${colors.border};display:flex;align-items:center;justify-content:center;font-size:24px">
                ${inst.icon || def?.icon || '📡'}
              </div>
              <div style="flex:1;min-width:0">
                <div style="font-size:14px;font-weight:600;color:var(--text);white-space:nowrap;overflow:hidden;text-overflow:ellipsis">${inst.name || def?.label}</div>
                <div style="font-size:11px;color:var(--text2);margin-top:2px">${def?.label || inst.channel_type}</div>
              </div>
            </div>

            ${!isAvailable && html`
              <div style="display:flex;flex-wrap:wrap;gap:6px;margin-bottom:10px">
                ${isActive ? html`<span style="font-size:11px;padding:3px 8px;border-radius:6px;background:rgba(34,197,94,0.15);color:var(--green);font-weight:500">● Hoạt động</span>`
                  : html`<span style="font-size:11px;padding:3px 8px;border-radius:6px;background:var(--bg3);color:var(--text2)">○ Tắt</span>`}
                ${inst.agent_name && html`<span style="font-size:11px;padding:3px 8px;border-radius:6px;background:rgba(139,92,246,0.15);color:var(--accent)">🤖 ${inst.agent_name}</span>`}
              </div>
            `}

            ${isAvailable ? html`
              <div style="text-align:center;padding:8px 0;color:var(--text2);font-size:12px">
                Click để thêm ${def?.label}
              </div>
            ` : html`
              <div style="display:flex;gap:6px;justify-content:flex-end;margin-top:4px" onClick=${e => e.stopPropagation()}>
                ${def?.fields && html`<button class="btn btn-outline btn-sm" onClick=${() => openConfig(inst)} style="font-size:11px;padding:4px 10px">⚙️ Cấu hình</button>`}
                ${inst.id && inst.id !== 'cli' && html`<button class="btn btn-outline btn-sm" onClick=${() => setDeleteConfirm(inst)} style="font-size:11px;padding:4px 10px;color:var(--red);border-color:var(--red)">🗑️</button>`}
              </div>
            `}
          </div>
        `;
      })}

      ${''}
      <div
        style="padding:20px;border-radius:14px;border:2px dashed var(--border);display:flex;flex-direction:column;align-items:center;justify-content:center;gap:8px;cursor:pointer;opacity:0.5;transition:all 0.3s"
        onClick=${() => setShowAddNew(true)}
        onMouseOver=${e => { e.currentTarget.style.opacity = '1'; e.currentTarget.style.borderColor = 'var(--accent)'; }}
        onMouseOut=${e => { e.currentTarget.style.opacity = '0.5'; e.currentTarget.style.borderColor = ''; }}>
        <div style="font-size:28px">➕</div>
        <div style="font-size:13px;color:var(--text2)">Thêm kênh mới</div>
      </div>
    </div>
  </div>`;
}

export { ChannelsPage };
