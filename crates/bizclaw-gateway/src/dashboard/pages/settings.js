// SettingsPage — extracted from app.js for modularity
// Uses window globals from index.html (Preact + HTM)
const { h, html, useState, useEffect, useContext, useCallback, useRef, useMemo } = window;
import { t, authFetch, authHeaders, StatsCard } from '/static/dashboard/shared.js';

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
  const [providersList, setProvidersList] = useState([]);
  const [customProvider, setCustomProvider] = useState(false);
  const [customModel, setCustomModel] = useState(false);
  const [localModels, setLocalModels] = useState([]);
  // Compliance state (Easy AI Model)
  const [compliance, setCompliance] = useState({
    pdpa: false, data_residency: 'vietnam', audit_log: true,
    retention_days: 90, anonymize_pii: false, encrypt_at_rest: true,
    consent_tracking: false, dpo_email: ''
  });
  const inp = 'width:100%;padding:8px;margin-top:4px;background:var(--bg2);border:1px solid var(--border);border-radius:6px;color:var(--text);font-size:13px';

  useEffect(() => {
    const loadTimeout = setTimeout(() => setLoading(false), 8000); // Safety: never stuck loading > 8s
    (async () => {
      try {
        const [cfgRes, provRes] = await Promise.all([
          authFetch('/api/v1/config'),
          authFetch('/api/v1/providers'),
        ]);
        const d = await cfgRes.json();
        const provData = await provRes.json();
        setProvidersList(provData.providers || []);
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
          // Check if current provider/model exists in list
          const pList = provData.providers || [];
          if(d.default_provider && !pList.find(p => p.name === d.default_provider)) setCustomProvider(true);
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
      try { const rm=await authFetch('/api/v1/brain/models'); const dm=await rm.json(); setLocalModels(dm.models||[]); } catch(e) {}
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
  const deleteFile = async (fname, e) => {
    e && e.stopPropagation();
    if(!confirm('Xóa file "'+fname+'"? Hành động này không thể hoàn tác.')) return;
    try {
      const r = await authFetch('/api/v1/brain/files/'+encodeURIComponent(fname), {method:'DELETE'});
      const d = await r.json();
      if(d.ok) { showToast('🗑️ Đã xóa: '+fname,'success'); if(editFile===fname){setEditFile(null);setFileContent('');} try{const r2=await authFetch('/api/v1/brain/files');const d2=await r2.json();setBrainFiles(d2.files||[]);}catch(ex){} }
      else showToast('❌ '+(d.error||'Lỗi'),'error');
    } catch(ex) { showToast('❌ '+ex.message,'error'); }
  };

  if(loading) return html`<div class="card" style="text-align:center;padding:40px;color:var(--text2)">Loading...</div>`;

  const tabs = [
    {id:'provider',icon:'🤖',label:'Nhà cung cấp AI'},
    {id:'identity',icon:'🪪',label:'Danh tính'},
    {id:'brain',icon:'🧠',label:'Brain Engine'},
    {id:'prompt',icon:'📝',label:'System Prompt'},
    {id:'interface',icon:'🖥️',label:'Giao diện'},
    {id:'compliance',icon:'🛡️',label:'Compliance'},
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
            <label>${t('set.provider',lang)}
              ${customProvider ? html`
                <div style="display:flex;gap:4px;margin-top:4px">
                  <input style="${inp};flex:1;margin-top:0" value=${form.provider} onInput=${e=>setForm(f=>({...f,provider:e.target.value}))} placeholder="custom-provider" />
                  <button class="btn btn-outline btn-sm" onClick=${()=>{setCustomProvider(false);if(providersList.length)setForm(f=>({...f,provider:providersList[0].name,model:(providersList[0].models||[])[0]||''}))}} title="Chọn từ danh sách">📋</button>
                </div>
              ` : html`
                <div style="display:flex;gap:4px;margin-top:4px">
                  <select style="${inp};flex:1;margin-top:0;cursor:pointer" value=${form.provider} onChange=${e=>{
                    const val=e.target.value;
                    if(val==='__custom__'){setCustomProvider(true);setForm(f=>({...f,provider:''}));return;}
                    const prov=providersList.find(p=>p.name===val);
                    setForm(f=>({...f,provider:val,model:(prov?.models||[])[0]||f.model}));
                    setCustomModel(false);
                  }}>
                    ${providersList.length===0?html`<option value="">— Chưa có provider —</option>`:''}
                    ${providersList.map(p=>html`<option key=${p.name} value=${p.name}>${p.icon||'🤖'} ${p.label||p.name} (${p.provider_type||''})</option>`)}
                    <option value="__custom__">✏️ Nhập thủ công...</option>
                  </select>
                </div>
              `}
            </label>
            <label>${t('set.model',lang)}
              ${customModel ? html`
                <div style="display:flex;gap:4px;margin-top:4px">
                  <input style="${inp};flex:1;margin-top:0" value=${form.model} onInput=${e=>setForm(f=>({...f,model:e.target.value}))} placeholder="model-name" />
                  <button class="btn btn-outline btn-sm" onClick=${()=>setCustomModel(false)} title="Chọn từ danh sách">📋</button>
                </div>
              ` : html`
                <div style="display:flex;gap:4px;margin-top:4px">
                  <select style="${inp};flex:1;margin-top:0;cursor:pointer" value=${form.model} onChange=${e=>{
                    if(e.target.value==='__custom__'){setCustomModel(true);setForm(f=>({...f,model:''}));return;}
                    setForm(f=>({...f,model:e.target.value}));
                  }}>
                    ${(()=>{
                      const prov=providersList.find(p=>p.name===form.provider);
                      let models=prov?.models||[];
                      if(form.provider==='brain' && localModels.length>0) {
                        models = localModels.map(m=>m.name);
                      }
                      if(models.length===0) return html`<option value=${form.model||''}>${form.model||'— Chọn model —'}</option>`;
                      return models.map(m=>html`<option key=${m} value=${m}>${m}</option>`);
                    })()}
                    <option value="__custom__">✏️ Nhập thủ công...</option>
                  </select>
                </div>
              `}
            </label>
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
            <div style="position:relative;width:44px;height:24px;background:${brainForm.enabled?'var(--green)':'var(--border)'};border-radius:12px;cursor:pointer;transition:background 0.3s" onClick=${()=>setBrainForm(f=>({...f,enabled:!f.enabled}))}>
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
                <label>Model Path (GGUF)
                  <select style="${inp};margin-bottom:4px;cursor:pointer" value=${localModels.find(m=>m.path===brainForm.model_path)?brainForm.model_path:''} onChange=${e=>{if(e.target.value)setBrainForm(f=>({...f,model_path:e.target.value}))}}>
                    <option value="">✏️ Nhập thủ công bên dưới...</option>
                    ${localModels.map(m=>html`<option key=${m.path} value=${m.path}>${m.name} (${m.size})</option>`)}
                  </select>
                  <input style="${inp};margin-top:0" value=${brainForm.model_path} onInput=${e=>setBrainForm(f=>({...f,model_path:e.target.value}))} placeholder="/models/qwen2-7b-q4.gguf" />
                </label>
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
          ${brainFiles.map(f=>{ const fname=f.filename||f.name||f; return html`<div key=${fname} style="display:flex;align-items:center;gap:8px;padding:8px 12px;background:var(--bg2);border-radius:6px;font-size:13px;cursor:pointer;border:1px solid transparent;transition:border-color .2s" onClick=${()=>openFile(fname)} onMouseOver=${e=>e.currentTarget.style.borderColor='var(--accent)'} onMouseOut=${e=>e.currentTarget.style.borderColor='transparent'}>
            <span style="font-size:16px">${f.is_custom?'📝':'📄'}</span>
            <div style="flex:1;min-width:0">
              <div style="font-weight:600">${fname}</div>
              ${f.section?html`<div style="font-size:10px;color:var(--text2);margin-top:1px">${f.section}</div>`:''}
            </div>
            <span style="color:var(--text2);font-size:11px;white-space:nowrap">${f.size?f.size+' B':''}</span>
            <span class="badge badge-blue" style="font-size:10px;cursor:pointer" onClick=${(e)=>{e.stopPropagation();openFile(fname);}}>✏️ Sửa</span>
            <span class="badge" style="font-size:10px;cursor:pointer;background:var(--red);color:#fff" onClick=${(e)=>deleteFile(fname,e)}>🗑️ Xóa</span>
          </div>`; })}
        </div>`}
      </div>
    `}

    ${tab==='prompt' && html`
      <div class="card"><div class="card-label">📝 System Prompt</div>
        <div style="font-size:12px;color:var(--text2);margin-bottom:10px">Hướng dẫn chung cho AI Agent — prompt này sẽ được gửi trước mỗi cuộc hội thoại.</div>
        <textarea style="width:100%;min-height:250px;padding:12px;background:var(--bg2);border:1px solid var(--border);border-radius:8px;color:var(--text);font-family:var(--mono);font-size:13px;resize:vertical;line-height:1.6" value=${form.sysprompt} onInput=${e=>setForm(f=>({...f,sysprompt:e.target.value}))} placeholder="You are a helpful AI assistant..." />
      </div>
    `}
    ${tab==='interface' && html`
      <div style="display:grid;grid-template-columns:1fr 1fr;gap:14px">
        <div class="card">
          <div class="card-label">🖥️ Giao diện người dùng</div>
          <p style="font-size:12px;color:var(--text2);margin:0 0 16px">Chọn chế độ giao diện phù hợp với trình độ kỹ thuật của bạn.</p>
          <div style="display:grid;gap:12px">
            <div class="sme-mode-toggle">
              <div class="label">
                <span class="icon">👑</span>
                <div>
                  <div>Chế độ SME</div>
                  <div class="desc">Giao diện đơn giản cho người dùng không chuyên kỹ thuật</div>
                </div>
              </div>
              <label class="switch">
                <input type="checkbox" checked=${localStorage.getItem('bizclaw_sme_mode') === 'true'} onChange=${e => {
                  const enabled = e.target.checked;
                  localStorage.setItem('bizclaw_sme_mode', enabled ? 'true' : 'false');
                  window.showToast(enabled ? '✅ Đã bật chế độ SME' : '✅ Đã bật chế độ Developer', 'success');
                  setTimeout(() => window.location.reload(), 500);
                }} />
                <span class="slider"></span>
              </label>
            </div>
          </div>
        </div>
        <div class="card">
          <div class="card-label">📚 Hướng dẫn chế độ</div>
          <div style="font-size:13px;color:var(--text2);line-height:1.8">
            <p style="margin-bottom:12px"><strong>👑 Chế độ SME:</strong> Hiển thị giao diện đơn giản với các tính năng cơ bản như Dashboard, Agents, Workflows và Reports. Phù hợp cho người dùng không cần các tùy chọn kỹ thuật chuyên sâu.</p>
            <p><strong>🔧 Chế độ Developer:</strong> Hiển thị đầy đủ các tính năng kỹ thuật bao gồm MCP Servers, Robot Hands, Config Files, FTS5 Search và nhiều hơn nữa.</p>
          </div>
        </div>
      </div>
    `}
    ${tab==='compliance' && html`
      <div style="display:grid;grid-template-columns:1fr 1fr;gap:14px">
        <div class="card">
          <div class="card-label">🛡️ Compliance & Tuân Thủ Dữ Liệu</div>
          <p style="font-size:12px;color:var(--text2);margin:0 0 16px">Cấu hình chính sách bảo vệ dữ liệu và tuân thủ quy định cho toàn bộ hệ thống AI Agent.</p>
          <div style="display:grid;gap:12px;font-size:13px">
            <!-- PDPA Toggle -->
            <div style="display:flex;align-items:center;justify-content:space-between;padding:12px;background:var(--bg2);border-radius:8px;border:1px solid var(--border)">
              <div>
                <strong>🇻🇳 PDPA (Bảo vệ Dữ liệu Cá nhân)</strong>
                <div style="font-size:11px;color:var(--text2);margin-top:2px">Tuân thủ Nghị định 13/2023/NĐ-CP</div>
              </div>
              <div style="position:relative;width:44px;height:24px;background:${compliance.pdpa?'var(--green)':'var(--border)'};border-radius:12px;cursor:pointer;transition:background 0.3s" onClick=${()=>setCompliance(c=>({...c,pdpa:!c.pdpa}))}>
                <div style="position:absolute;top:2px;left:${compliance.pdpa?'22px':'2px'};width:20px;height:20px;background:#fff;border-radius:50%;transition:left 0.3s;box-shadow:0 1px 3px rgba(0,0,0,0.3)"></div>
              </div>
            </div>
            <!-- Data Residency -->
            <label>Data Residency (Nơi lưu trữ dữ liệu)
              <select style="${inp}" value=${compliance.data_residency} onChange=${e=>setCompliance(c=>({...c,data_residency:e.target.value}))}>
                <option value="vietnam">🇻🇳 Việt Nam (On-premise / VPS nội địa)</option>
                <option value="singapore">🇸🇬 Singapore (Cloud SEA)</option>
                <option value="local">💻 Local Device (Privacy-First)</option>
                <option value="hybrid">🌐 Hybrid (Cloud + Local fallback)</option>
              </select>
            </label>
            <!-- Audit Log -->
            <div style="display:flex;align-items:center;justify-content:space-between;padding:12px;background:var(--bg2);border-radius:8px;border:1px solid var(--border)">
              <div>
                <strong>📝 Audit Log (Nhật ký Kiểm toán)</strong>
                <div style="font-size:11px;color:var(--text2);margin-top:2px">Ghi lại mọi thao tác của Agent và admin</div>
              </div>
              <div style="position:relative;width:44px;height:24px;background:${compliance.audit_log?'var(--green)':'var(--border)'};border-radius:12px;cursor:pointer;transition:background 0.3s" onClick=${()=>setCompliance(c=>({...c,audit_log:!c.audit_log}))}>
                <div style="position:absolute;top:2px;left:${compliance.audit_log?'22px':'2px'};width:20px;height:20px;background:#fff;border-radius:50%;transition:left 0.3s;box-shadow:0 1px 3px rgba(0,0,0,0.3)"></div>
              </div>
            </div>
            <!-- Data Retention -->
            <label>Data Retention (Định kỳ xóa dữ liệu cũ)
              <select style="${inp}" value=${compliance.retention_days} onChange=${e=>setCompliance(c=>({...c,retention_days:+e.target.value}))}>
                <option value="30">30 ngày</option>
                <option value="90">90 ngày (Khuyến nghị)</option>
                <option value="180">180 ngày</option>
                <option value="365">365 ngày</option>
                <option value="0">Vĩnh viễn (Không xóa)</option>
              </select>
            </label>
            <!-- PII Anonymization -->
            <div style="display:flex;align-items:center;justify-content:space-between;padding:12px;background:var(--bg2);border-radius:8px;border:1px solid var(--border)">
              <div>
                <strong>🔒 Anonymize PII (Mã hóa SĐT/CMND)</strong>
                <div style="font-size:11px;color:var(--text2);margin-top:2px">Tự động mask số điện thoại, CMND trong log</div>
              </div>
              <div style="position:relative;width:44px;height:24px;background:${compliance.anonymize_pii?'var(--green)':'var(--border)'};border-radius:12px;cursor:pointer;transition:background 0.3s" onClick=${()=>setCompliance(c=>({...c,anonymize_pii:!c.anonymize_pii}))}>
                <div style="position:absolute;top:2px;left:${compliance.anonymize_pii?'22px':'2px'};width:20px;height:20px;background:#fff;border-radius:50%;transition:left 0.3s;box-shadow:0 1px 3px rgba(0,0,0,0.3)"></div>
              </div>
            </div>
            <!-- DPO Contact -->
            <label>DPO Email (Liên hệ Bảo vệ Dữ liệu)
              <input style="${inp}" value=${compliance.dpo_email} onInput=${e=>setCompliance(c=>({...c,dpo_email:e.target.value}))} placeholder="dpo@company.com" />
            </label>
          </div>
        </div>
        <div class="card">
          <div class="card-label">🏅 Compliance Scorecard</div>
          <div style="display:grid;gap:8px;font-size:13px">
            ${[
              { label: 'PDPA Việt Nam', ok: compliance.pdpa, icon: '🇻🇳' },
              { label: 'Data Residency', ok: compliance.data_residency !== '', icon: '📡' },
              { label: 'Audit Logging', ok: compliance.audit_log, icon: '📝' },
              { label: 'Data Retention Policy', ok: compliance.retention_days > 0, icon: '⏰' },
              { label: 'PII Anonymization', ok: compliance.anonymize_pii, icon: '🔒' },
              { label: 'Encryption at Rest', ok: compliance.encrypt_at_rest, icon: '🛡️' },
              { label: 'DPO Contact', ok: !!compliance.dpo_email, icon: '📧' },
            ].map(item => html`
              <div key=${item.label} style="display:flex;align-items:center;gap:10px;padding:10px 14px;background:var(--bg2);border-radius:8px;border-left:3px solid ${item.ok ? 'var(--green)' : 'var(--red)'}">
                <span style="font-size:18px">${item.icon}</span>
                <span style="flex:1;font-weight:500">${item.label}</span>
                <span class="badge ${item.ok ? 'badge-green' : 'badge-red'}" style="font-size:10px">${item.ok ? '✅ OK' : '⚠️ Chưa'}</span>
              </div>
            `)}
            <div style="margin-top:8px;padding:14px;background:${[compliance.pdpa,compliance.audit_log,compliance.anonymize_pii,compliance.encrypt_at_rest,!!compliance.dpo_email].filter(Boolean).length >= 4 ? 'rgba(16,185,129,0.08)' : 'rgba(239,68,68,0.06)'};border-radius:10px;text-align:center">
              <div style="font-size:28px;font-weight:800;color:${[compliance.pdpa,compliance.audit_log,compliance.anonymize_pii,compliance.encrypt_at_rest,!!compliance.dpo_email].filter(Boolean).length >= 4 ? 'var(--green)' : 'var(--red)'}">
                ${Math.round([compliance.pdpa,compliance.audit_log,compliance.anonymize_pii,compliance.encrypt_at_rest,!!compliance.dpo_email,compliance.retention_days>0,compliance.data_residency!==''].filter(Boolean).length / 7 * 100)}%
              </div>
              <div style="font-size:12px;color:var(--text2);margin-top:4px">Độ tuân thủ tổng thể</div>
            </div>
          </div>
        </div>
      </div>
    `}
  </div>`;
}


export { SettingsPage };
