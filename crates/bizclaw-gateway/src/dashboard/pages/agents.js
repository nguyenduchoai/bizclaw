// AgentsPage — extracted from app.js for modularity
// Uses window globals from index.html (Preact + HTM)
const { h, html, useState, useEffect, useContext, useCallback, useRef, useMemo } = window;
import { t, authFetch, authHeaders, StatsCard } from '/static/dashboard/shared.js';

function AgentsPage({ config, lang }) {
  const { showToast } = useContext(AppContext);
  const [agents,setAgents] = useState([]);
  const [loading,setLoading] = useState(true);
  const [showForm,setShowForm] = useState(false);
  const [editAgent,setEditAgent] = useState(null);
  const [form,setForm] = useState({name:'',role:'',description:'',system_prompt:'',provider:'',model:'',channels:[], rag_collection:'', sql_connection:''});
  const availableChannels = ['telegram','zalo','discord','webhook','web'];
  const [providersList, setProvidersList] = useState([]);
  const [collections, setCollections] = useState([]);
  const [sqlConns, setSqlConns] = useState([]);
  const [customAgentProv, setCustomAgentProv] = useState(false);
  const [customAgentModel, setCustomAgentModel] = useState(false);
  const [localModels, setLocalModels] = useState([]);

  const load = async () => {
    try {
      const [agRes, provRes, docsRes, sqlRes] = await Promise.all([
        authFetch('/api/v1/agents'),
        authFetch('/api/v1/providers'),
        authFetch('/api/v1/knowledge/documents'),
        authFetch('/api/v1/nl-query/status')
      ]);
      const agData = await agRes.json();
      const provData = await provRes.json();
      const docsData = await docsRes.json();
      const sqlData = await sqlRes.json();

      setAgents(agData.agents || []);
      setProvidersList(provData.providers || []);
      
      // Extract unique collections from documents array or generic list
      let s = new Set();
      if (docsData.documents) {
        docsData.documents.map(d => d.owner).filter(Boolean).forEach(o => s.add(o));
      }
      try {
        const colRes = await authFetch('/api/v1/brain/files/rag_collections.json');
        if (colRes.ok) {
          const d = await colRes.json();
          (d.collections || []).forEach(o => s.add(o));
        }
      } catch(e) {}
      setCollections(Array.from(s).sort());
      if (sqlData.connections) {
        setSqlConns(sqlData.connections || []);
      }
      try { const rm=await authFetch('/api/v1/brain/models'); const dm=await rm.json(); setLocalModels(dm.models||[]); } catch(e) {}
    } catch(e){ console.error('AgentsPage load error:', e); }
    setLoading(false);
  };
  useEffect(()=>{ load(); },[]);

  const openCreate = () => { setEditAgent(null); setCustomAgentProv(false); setCustomAgentModel(false); setForm({name:'',role:'general',description:'',system_prompt:'',provider:config?.default_provider||'',model:config?.default_model||'',channels:[], rag_collection:'', sql_connection:''}); setShowForm(true); };
  const openEdit = (a) => {
    setEditAgent(a);
    setForm({name:a.name,role:a.role||'',description:a.description||'',system_prompt:a.system_prompt||'',provider:a.provider||'',model:a.model||'',channels:a.channels||[], rag_collection:a.rag_collection||'', sql_connection:a.sql_connection||''});
    // Check if provider/model exists in list
    setCustomAgentProv(a.provider && !providersList.find(p => p.name === a.provider));
    setCustomAgentModel(false);
    setShowForm(true);
  };

  const saveAgent = async () => {
    try {
      const agentData = {name:form.name,role:form.role,description:form.description,system_prompt:form.system_prompt,provider:form.provider,model:form.model, rag_collection: form.rag_collection, sql_connection: form.sql_connection};
      if(editAgent) {
        const r = await authFetch('/api/v1/agents/'+encodeURIComponent(editAgent.name), {
          method:'PUT', headers:{'Content-Type':'application/json'}, body:JSON.stringify(agentData)
        });
        const d=await r.json();
        if(d.ok) {
          // Save channel bindings
          if((form.channels||[]).length > 0 || (editAgent.channels||[]).length > 0) {
            await authFetch('/api/v1/agents/'+encodeURIComponent(form.name)+'/channels', {
              method:'PUT', headers:{'Content-Type':'application/json'}, body:JSON.stringify({channels:form.channels||[]})
            });
          }
          showToast('✅ Đã cập nhật agent: '+form.name,'success'); load(); setShowForm(false);
        }
        else showToast('❌ '+(d.error||'Lỗi'),'error');
      } else {
        const r = await authFetch('/api/v1/agents', {
          method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify(agentData)
        });
        const d=await r.json();
        if(d.ok) {
          // Save channel bindings for new agent
          if((form.channels||[]).length > 0) {
            await authFetch('/api/v1/agents/'+encodeURIComponent(form.name)+'/channels', {
              method:'PUT', headers:{'Content-Type':'application/json'}, body:JSON.stringify({channels:form.channels||[]})
            });
          }
          showToast('✅ Đã tạo agent: '+form.name,'success'); load(); setShowForm(false);
        }
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
          <label>Provider
            ${customAgentProv ? html`
              <div style="display:flex;gap:4px;margin-top:4px">
                <input style="${inp};flex:1;margin-top:0" value=${form.provider} onInput=${e=>setForm(f=>({...f,provider:e.target.value}))} placeholder="custom-provider" />
                <button class="btn btn-outline btn-sm" onClick=${()=>{setCustomAgentProv(false);if(providersList.length)setForm(f=>({...f,provider:providersList[0].name,model:(providersList[0].models||[])[0]||''}))}} title="Chọn từ danh sách">📋</button>
              </div>
            ` : html`
              <select style="${inp};cursor:pointer" value=${form.provider} onChange=${e=>{
                if(e.target.value==='__custom__'){setCustomAgentProv(true);setForm(f=>({...f,provider:''}));return;}
                const prov=providersList.find(p=>p.name===e.target.value);
                setForm(f=>({...f,provider:e.target.value,model:(prov?.models||[])[0]||f.model}));
                setCustomAgentModel(false);
              }}>
                <option value="">— Chọn Provider —</option>
                ${providersList.map(p=>html`<option key=${p.name} value=${p.name}>${p.icon||'🤖'} ${p.label||p.name}</option>`)}
                <option value="__custom__">✏️ Nhập thủ công...</option>
              </select>
            `}
          </label>
          <label>Model
            ${customAgentModel ? html`
              <div style="display:flex;gap:4px;margin-top:4px">
                <input style="${inp};flex:1;margin-top:0" value=${form.model} onInput=${e=>setForm(f=>({...f,model:e.target.value}))} placeholder="model-name" />
                <button class="btn btn-outline btn-sm" onClick=${()=>setCustomAgentModel(false)} title="Chọn từ danh sách">📋</button>
              </div>
            ` : html`
              <div style="display:flex;gap:4px;align-items:center;margin-top:4px">
                <select style="${inp};flex:1;margin-top:0;cursor:pointer" value=${form.model} onChange=${e=>{
                  if(e.target.value==='__custom__'){setCustomAgentModel(true);setForm(f=>({...f,model:''}));return;}
                  setForm(f=>({...f,model:e.target.value}));
                }}>
                  <option value="">— Chọn Model —</option>
                  ${(()=>{
                    const prov=providersList.find(p=>p.name===form.provider);
                    let models=prov?.models||[];
                    if(form.provider==='brain' && localModels.length>0) {
                      models = localModels.map(m=>m.name);
                    }
                    return models.map(m=>html`<option key=${m} value=${m}>${m}</option>`);
                  })()}
                  <option value="__custom__">✏️ Nhập thủ công...</option>
                </select>
                <button class="btn btn-outline btn-sm" style="padding:4px 8px" onClick=${async ()=>{
                  if(!form.provider) return;
                  showToast('⏳ Đang cập nhật models...', 'info');
                  try {
                    const r = await authFetch('/api/v1/providers/' + encodeURIComponent(form.provider) + '/fetch-models', { method: 'POST' });
                    const d = await r.json();
                    if(d.ok) {
                      showToast('✅ Đã cập nhật models', 'success');
                      // Refresh providers list
                      const pr = await authFetch('/api/v1/providers');
                      const pd = await pr.json();
                      setProvidersList(pd.providers || []);
                    } else {
                      showToast('❌ ' + (d.error || 'Lỗi'), 'error');
                    }
                  } catch(e) { showToast('❌ ' + e.message, 'error'); }
                }} title="Cập nhật danh sách model từ nhà cung cấp">🔄</button>
              </div>
            `}
          </label>
          <label style="grid-column:span 2">Mô tả<input style="${inp}" value=${form.description} onInput=${e=>setForm(f=>({...f,description:e.target.value}))} placeholder="Mô tả ngắn..." /></label>
          <label>Kho Nội Bộ (PDF/Doc) (Auto-RAG)
            <select style="${inp}" value=${form.rag_collection} onChange=${e=>setForm(f=>({...f,rag_collection:e.target.value}))}>
              <option value="">— Mặc định (Tất cả) —</option>
              ${collections.map(c => html`<option key=${c} value=${c}>📚 ${c}</option>`)}
            </select>
          </label>
          <label>Nguồn Dữ Liệu SQL
            <select style="${inp}" value=${form.sql_connection} onChange=${e=>setForm(f=>({...f,sql_connection:e.target.value}))}>
              <option value="">— Không kết nối —</option>
              ${sqlConns.map(c => html`<option key=${c.id} value=${c.id}>🗄️ ${c.id} (${c.db_type})</option>`)}
            </select>
          </label>
          <label style="grid-column:span 2">System Prompt<textarea style="${inp};min-height:80px;resize:vertical;font-family:var(--mono)" value=${form.system_prompt} onInput=${e=>setForm(f=>({...f,system_prompt:e.target.value}))} placeholder="You are a..." /></label>
          <label style="grid-column:span 2">📡 Gán Agent với Kênh
            <div style="display:flex;gap:8px;flex-wrap:wrap;margin-top:6px">
              ${availableChannels.map(ch => {
                const icons = {telegram:'📱',zalo:'💙',discord:'💬',webhook:'🌐',web:'🖥️'};
                const labels = {telegram:'Telegram',zalo:'Zalo',discord:'Discord',webhook:'Webhook',web:'Web Chat'};
                const active = (form.channels||[]).includes(ch);
                return html`<button key=${ch} type="button" class="btn btn-sm ${active?'':'btn-outline'}" style="${active?'background:var(--accent);color:#fff;border-color:var(--accent)':''}" onClick=${()=>{
                  setForm(f => ({...f, channels: active ? (f.channels||[]).filter(c=>c!==ch) : [...(f.channels||[]),ch]}));
                }}>${icons[ch]||'📡'} ${labels[ch]||ch}</button>`;
              })}
            </div>
            <div style="font-size:10px;color:var(--text2);margin-top:4px">Chọn kênh mà agent này sẽ tự động trả lời. Có thể chọn nhiều kênh.</div>
          </label>
        </div>
        <div style="margin-top:12px;display:flex;gap:8px;justify-content:flex-end">
          <button class="btn btn-outline" onClick=${()=>setShowForm(false)}>Huỷ</button>
          <button class="btn" style="background:var(--grad1);color:#fff;padding:8px 20px" onClick=${saveAgent}>💾 ${editAgent?'Cập nhật':'Tạo'}</button>
        </div>
      </div>
    `}

    <div class="card">${loading?html`<div style="text-align:center;padding:20px;color:var(--text2)">Loading...</div>`:html`
      <table><thead><tr><th>Agent</th><th>Vai trò</th><th>Provider</th><th>Model</th><th>Channels</th><th>Messages</th><th>Status</th><th style="text-align:right">Thao tác</th></tr></thead><tbody>
        
        <!-- Pinned: Default Agent (AI Mama / Orchestrator) -->
        <tr style="background:var(--bg2)">
          <td><strong>${config?.agent_name||'BizClaw'} 👑</strong><div style="font-size:11px;color:var(--text2)">AI Mama tổng quản (Orchestrator). Nhận tin nhắn và điều phối.</div></td>
          <td><span class="badge badge-purple">Orchestrator</span></td>
          <td>${config?.default_provider||'—'}</td>
          <td><span class="badge badge-blue">${config?.default_model||'Mặc định'}</span></td>
          <td><span class="badge" style="font-size:10px">📡 Toàn hệ thống</span></td>
          <td>—</td>
          <td><span class="badge badge-green">Core Active</span></td>
          <td style="text-align:right">
            <a href="/brain" class="btn btn-outline btn-sm" title="Sửa file não bộ (SOUL.md)">🧠 Cấu hình não bộ</a>
          </td>
        </tr>

        ${agents.map(a=>html`<tr key=${a.name||a.id}>
          <td><strong>${a.name}</strong>${a.description?html`<div style="font-size:11px;color:var(--text2)">${a.description}</div>`:''}</td>
          <td><span class="badge">${a.role||'—'}</span></td>
          <td>${a.provider||'—'}</td>
          <td><span class="badge badge-blue">${a.model||'—'}</span></td>
          <td>${(a.channels||[]).length>0 ? (a.channels||[]).map(ch=>html`<span key=${ch} class="badge" style="margin-right:2px;font-size:10px">${{telegram:'📱',zalo:'💙',discord:'💬',webhook:'🌐',web:'🖥️'}[ch]||'📡'} ${ch}</span>`) : html`<span style="color:var(--text2);font-size:11px">—</span>`}</td>
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


export { AgentsPage };
