// ProvidersPage — extracted from app.js for modularity
// Uses window globals from index.html (Preact + HTM)
const { h, html, useState, useEffect, useContext, useCallback, useRef, useMemo } = window;
import { t, authFetch, authHeaders, StatsCard } from '/static/dashboard/shared.js';

function ProvidersPage({ config, lang }) {
  const { showToast } = useContext(AppContext);
  const [providers, setProviders] = useState([]);
  const [loading, setLoading] = useState(true);
  const [configProv, setConfigProv] = useState(null);
  const [provForm, setProvForm] = useState({api_key:'',base_url:'',model:''});
  const [showCreate, setShowCreate] = useState(false);
  const [createForm, setCreateForm] = useState({name:'',label:'',api_key:'',base_url:'',model:'',provider_type:'cloud'});

  const load = async () => {
    try { const r=await authFetch('/api/v1/providers'); const d=await r.json(); setProviders(d.providers||[]); } catch(e){}
    setLoading(false);
  };
  useEffect(()=>{ load(); },[]);

  const active = config?.default_provider || '';
  const typeColor = t => t==='cloud'?'badge-blue':t==='local'?'badge-green':'badge-purple';

  const [brainModels, setBrainModels] = useState([]);
  const [downloading, setDownloading] = useState({});

  const loadBrainModels = async () => {
    try {
      const r = await authFetch('/api/v1/brain/models');
      const d = await r.json();
      if(d.ok) setBrainModels(d.models || []);
    } catch(e) {}
  };

  const downloadBrainModel = async (modelId) => {
    try {
      setDownloading(prev => ({...prev, [modelId]: { status: 'starting', percent: 0 }}));
      const r = await authFetch('/api/v1/brain/models/download', {
        method: 'POST', headers: {'Content-Type': 'application/json'},
        body: JSON.stringify({ model: modelId })
      });
      const d = await r.json();
      if(d.ok) {
        showToast('⏳ Bắt đầu tải ' + modelId, 'success');
        pollDownloadStatus(modelId);
      } else {
        showToast('❌ ' + (d.error || 'Lỗi tải model'), 'error');
        setDownloading(prev => { const n = {...prev}; delete n[modelId]; return n; });
      }
    } catch(e) { showToast('❌ ' + e.message, 'error'); }
  };

  const pollDownloadStatus = async (modelId) => {
    // Basic polling
    const interval = setInterval(async () => {
      try {
        const fileExt = modelId.includes('e2b') || modelId.includes('e4b') || modelId.includes('26b') ? 'gemma-4-' + modelId.split('-')[1].toUpperCase() + '-it-Q4_K_M.gguf' : modelId + '.gguf';
        const r = await authFetch('/api/v1/brain/models/status/' + encodeURIComponent(fileExt));
        const d = await r.json();
        if(d.status === 'completed') {
          clearInterval(interval);
          setDownloading(prev => { const n = {...prev}; delete n[modelId]; return n; });
          showToast('✅ Tải xong ' + modelId, 'success');
          loadBrainModels();
        } else if(d.status === 'downloading' && d.progress) {
          setDownloading(prev => ({...prev, [modelId]: { status: 'downloading', percent: d.progress.percent || 0 }}));
        }
      } catch(e) {}
    }, 2000);
  };

  const deleteBrainModel = async (filename) => {
    if(!confirm('Xoá file model "'+filename+'"?')) return;
    try {
      const r = await authFetch('/api/v1/brain/models/'+encodeURIComponent(filename), {method:'DELETE'});
      const d = await r.json();
      if(d.ok) { showToast('🗑️ Đã xoá: '+filename,'success'); loadBrainModels(); load(); }
      else showToast('❌ '+(d.error||'Lỗi xoá'),'error');
    } catch(e) { showToast('❌ '+e.message,'error'); }
  };

  const recommendedModels = [
    { id: 'gemma4-e2b', name: 'Gemma 4 E2B (~1.5GB)', desc: 'Nhỏ gọn, chạy tốt trên máy yếu/Android' },
    { id: 'gemma4-e4b', name: 'Gemma 4 E4B (~2.8GB)', desc: 'Cân bằng tốc độ & độ thông minh' },
    { id: 'gemma4-26b', name: 'Gemma 4 26B (~14GB)', desc: 'Siêu việt, cần phần cứng khoẻ (GPU)' },
    { id: 'llama-3.2-1b', name: 'Llama 3.2 1B (~750MB)', desc: 'Rất nhẹ' },
    { id: 'phi-2', name: 'Phi-2 (~1.6GB)', desc: 'Xử lý toán & logic tốt' }
  ];

  // Modified openConfig to also trigger loadBrainModels
  const openConfig = (p) => {
    setConfigProv(p);
    setProvForm({api_key:p.api_key||'',base_url:p.base_url||'',model:(p.models||[])[0]||''});
    if(p.name === 'brain') loadBrainModels();
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

  const createProvider = async () => {
    if(!createForm.name.trim()) { showToast('⚠️ Nhập tên provider','error'); return; }
    try {
      const r = await authFetch('/api/v1/providers', {
        method:'POST', headers:{'Content-Type':'application/json'},
        body:JSON.stringify(createForm)
      });
      const d=await r.json();
      if(d.ok) { showToast('✅ Đã tạo provider: '+createForm.name,'success'); setShowCreate(false); setCreateForm({name:'',label:'',api_key:'',base_url:'',model:'',provider_type:'cloud'}); load(); }
      else showToast('❌ '+(d.error||'Lỗi'),'error');
    } catch(e) { showToast('❌ '+e.message,'error'); }
  };

  const deleteProvider = async (name) => {
    if(!confirm('Xoá provider "'+name+'"?')) return;
    try {
      const r = await authFetch('/api/v1/providers/'+encodeURIComponent(name), {method:'DELETE'});
      const d=await r.json();
      if(d.ok) { showToast('🗑️ Đã xoá: '+name,'success'); load(); }
      else showToast('❌ '+(d.error||'Lỗi'),'error');
    } catch(e) { showToast('❌ '+e.message,'error'); }
  };

  const refreshModels = async (name) => {
    try {
      showToast('⏳ Đang cập nhật model cho ' + name + '...', 'info');
      const r = await authFetch('/api/v1/providers/' + encodeURIComponent(name) + '/fetch-models', { method: 'POST' });
      const d = await r.json();
      if (d.ok) {
        showToast('✅ Đã cập nhật ' + (d.models?.length || 0) + ' models cho ' + name, 'success');
        load(); // Reload provider list to show new models
      } else {
        showToast('❌ ' + (d.error || 'Lỗi cập nhật model'), 'error');
      }
    } catch (e) {
      showToast('❌ ' + e.message, 'error');
    }
  };

  const inp = 'width:100%;padding:8px;margin-top:4px;background:var(--bg2);border:1px solid var(--border);border-radius:6px;color:var(--text);font-size:13px';

  return html`<div>
    <div class="page-header"><div><h1>🔌 ${t('providers.title',lang)}</h1><div class="sub">${t('providers.subtitle',lang)}</div></div>
      <button class="btn" style="background:var(--grad1);color:#fff;padding:8px 18px" onClick=${()=>setShowCreate(!showCreate)}>+ Thêm Provider</button>
    </div>
    <div class="stats">
      <${StatsCard} label=${t('providers.active_label',lang)} value=${active||'—'} color="green" icon="⚡" />
      <${StatsCard} label=${t('providers.total_label',lang)} value=${providers.length} color="accent" icon="🔌" />
    </div>

    ${showCreate && html`
      <div class="card" style="margin-bottom:14px;border:1px solid var(--accent)">
        <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px">
          <h3>🔌 Thêm Provider mới</h3>
          <button class="btn btn-outline btn-sm" onClick=${()=>setShowCreate(false)}>✕ Đóng</button>
        </div>
        <div style="display:grid;grid-template-columns:1fr 1fr;gap:10px;font-size:13px">
          <label>Tên (ID)<input style="${inp}" value=${createForm.name} onInput=${e=>setCreateForm(f=>({...f,name:e.target.value}))} placeholder="my-openai" /></label>
          <label>Label<input style="${inp}" value=${createForm.label} onInput=${e=>setCreateForm(f=>({...f,label:e.target.value}))} placeholder="My OpenAI" /></label>
          <label>API Key<input style="${inp}" type="password" value=${createForm.api_key} onInput=${e=>setCreateForm(f=>({...f,api_key:e.target.value}))} placeholder="sk-..." /></label>
          <label>Base URL<input style="${inp}" value=${createForm.base_url} onInput=${e=>setCreateForm(f=>({...f,base_url:e.target.value}))} placeholder="https://api.openai.com/v1" /></label>
          <label>Default Model<input style="${inp}" value=${createForm.model} onInput=${e=>setCreateForm(f=>({...f,model:e.target.value}))} placeholder="gpt-4o" /></label>
          <label>Type
            <select style="${inp};cursor:pointer" value=${createForm.provider_type} onChange=${e=>setCreateForm(f=>({...f,provider_type:e.target.value}))}>
              <option value="cloud">Cloud</option><option value="local">Local</option><option value="proxy">Proxy</option>
            </select>
          </label>
        </div>
        <div style="margin-top:14px;display:flex;gap:8px;justify-content:flex-end">
          <button class="btn btn-outline" onClick=${()=>setShowCreate(false)}>Huỷ</button>
          <button class="btn" style="background:var(--grad1);color:#fff;padding:8px 20px" onClick=${createProvider}>💾 Tạo</button>
        </div>
      </div>
    `}

    ${configProv && html`
      <div class="card" style="margin-bottom:14px;border:1px solid var(--accent)">
        <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px">
          <h3>🔑 Cấu hình ${configProv.label||configProv.name}</h3>
          <button class="btn btn-outline btn-sm" onClick=${()=>setConfigProv(null)}>✕ Đóng</button>
        </div>
        
        ${configProv.name === 'brain' && html`
          <div style="margin-bottom:20px;padding:12px;background:var(--bg2);border-radius:8px;border:1px solid var(--border)">
            <h4 style="margin-top:0;margin-bottom:10px;display:flex;align-items:center;gap:6px">🧠 Quản lý Local Models (GGUF)</h4>
            
            <div style="display:grid;grid-template-columns:1fr 1fr;gap:12px;margin-bottom:14px">
              ${recommendedModels.map(rm => {
                const dl = downloading[rm.id];
                return html`
                  <div key=${rm.id} style="padding:10px;background:var(--surface);border:1px solid var(--border);border-radius:8px;display:flex;justify-content:space-between;align-items:center">
                    <div>
                      <div style="font-weight:600;font-size:13px">${rm.name}</div>
                      <div style="font-size:11px;color:var(--text2)">${rm.desc}</div>
                    </div>
                    ${dl ? html`
                      <div style="font-size:12px;color:var(--accent);font-weight:500;text-align:right">
                        <div>Đang tải... ${dl.percent}%</div>
                        <div style="width:60px;height:4px;background:var(--bg3);border-radius:2px;margin-top:4px;overflow:hidden">
                          <div style="height:100%;width:${dl.percent}%;background:var(--accent)"></div>
                        </div>
                      </div>
                    ` : html`
                      <button class="btn btn-sm" style="background:var(--bg3);color:var(--text)" onClick=${() => downloadBrainModel(rm.id)}>⬇️ Tải</button>
                    `}
                  </div>
                `;
              })}
            </div>
            
            <h5 style="margin-top:0;margin-bottom:8px">Đã tải về máy:</h5>
            ${brainModels.length === 0 ? html`<div style="font-size:12px;color:var(--text2)">Chưa có model nào.</div>` : html`
              <table style="font-size:13px">
                <tbody>
                  ${brainModels.map(m => html`
                    <tr key=${m.name}>
                      <td style="padding:6px 0"><strong>${m.name}</strong></td>
                      <td style="padding:6px 0;text-align:right;color:var(--text2)">${m.size}</td>
                      <td style="padding:6px 0;text-align:right">
                        <button class="btn btn-outline btn-sm" style="color:var(--red);border-color:var(--red);padding:2px 8px" onClick=${() => deleteBrainModel(m.name)}>Xoá</button>
                      </td>
                    </tr>
                  `)}
                </tbody>
              </table>
            `}
          </div>
        `}

        <div style="display:grid;grid-template-columns:1fr 1fr;gap:10px;font-size:13px">
          ${configProv.name !== 'brain' && html`
            <label>API Key<input style="${inp}" type="password" value=${provForm.api_key} onInput=${e=>setProvForm(f=>({...f,api_key:e.target.value}))} placeholder="sk-..." /></label>
            <label>Base URL<input style="${inp}" value=${provForm.base_url} onInput=${e=>setProvForm(f=>({...f,base_url:e.target.value}))} placeholder="https://api.openai.com/v1" /></label>
          `}
          <label style="grid-column:span 2">Default Model
            ${configProv.name === 'brain' ? html`
              <select style="${inp};cursor:pointer" value=${provForm.model} onChange=${e=>setProvForm(f=>({...f,model:e.target.value}))}>
                <option value="">— Chọn Model —</option>
                ${brainModels.map(m => html`<option key=${m.name} value=${m.name}>${m.name} (${m.size})</option>`)}
              </select>
            ` : html`
              <input style="${inp}" value=${provForm.model} onInput=${e=>setProvForm(f=>({...f,model:e.target.value}))} placeholder="gpt-4o, llama3.2..." />
            `}
          </label>
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
          ${p.name !== 'brain' && html`<button class="btn btn-outline btn-sm" onClick=${()=>refreshModels(p.name)} title="Cập nhật Model">🔄</button>`}
          <button class="btn btn-outline btn-sm" style="margin-left:4px" onClick=${()=>openConfig(p)} title="Cấu hình">🔑</button>
          ${p.name!==active?html`<button class="btn btn-outline btn-sm" style="margin-left:4px" onClick=${()=>activateProvider(p.name,(p.models||[])[0])} title="Kích hoạt">⚡</button>`:''}
          <button class="btn btn-outline btn-sm" style="margin-left:4px;color:var(--red)" onClick=${()=>deleteProvider(p.name)} title="Xoá">🗑️</button>
        </td>
      </tr>`)}
    </tbody></table>`}</div>
  </div>`;
}


export { ProvidersPage };
