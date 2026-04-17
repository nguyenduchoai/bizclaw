// ═══════════════════════════════════════════════════════════════
// BizClaw — TikTok Shop & Creator Dashboard
// Phase 1: TikTok Shop, Video Upload, Content Creation
// ═══════════════════════════════════════════════════════════════
const { h, html, useState, useEffect, useContext, useCallback, useRef, useMemo } = window;
const { authFetch, t, StatsCard } = window;

// ── API Base ──
const API = '/api/v1/tiktok';

function TikTokPage({ lang }) {
  const { showToast, navigate } = useContext(AppContext);
  
  // ── State ──
  const [activeTab, setActiveTab] = useState('shop'); // shop | video | content | settings
  const [loading, setLoading] = useState(true);
  const [config, setConfig] = useState(null);
  const [orders, setOrders] = useState([]);
  const [products, setProducts] = useState([]);
  const [videos, setVideos] = useState([]);
  const [uploading, setUploading] = useState(false);
  const [uploadProgress, setUploadProgress] = useState(0);

  // ── Config Form ──
  const [configForm, setConfigForm] = useState({
    app_id: '',
    app_secret: '',
    access_token: '',
    shop_id: '',
  });

  // ── Video Upload Form ──
  const [videoForm, setVideoForm] = useState({
    title: '',
    description: '',
    privacy_level: 'public',
    brand_category: '',
    for_fyp: true,
    video_file: null,
  });

  const inp = 'width:100%;padding:8px;margin-top:4px;background:var(--bg2);border:1px solid var(--border);border-radius:6px;color:var(--text);font-size:13px';

  // ── Load Data ──
  const loadData = useCallback(async () => {
    setLoading(true);
    try {
      const [configRes, ordersRes, productsRes, videosRes] = await Promise.all([
        authFetch(`${API}/config`),
        authFetch(`${API}/orders`),
        authFetch(`${API}/products`),
        authFetch(`${API}/videos`),
      ]);

      if (configRes.ok) {
        const d = await configRes.json();
        setConfig(d);
        setConfigForm({
          app_id: d.app_id || '',
          app_secret: d.app_secret || '',
          access_token: d.access_token || '',
          shop_id: d.shop_id || '',
        });
      }
      if (ordersRes.ok) {
        const d = await ordersRes.json();
        setOrders(d.orders || []);
      }
      if (productsRes.ok) {
        const d = await productsRes.json();
        setProducts(d.products || []);
      }
      if (videosRes.ok) {
        const d = await videosRes.json();
        setVideos(d.videos || []);
      }
    } catch(e) {
      console.error('TikTok load error:', e);
    }
    setLoading(false);
  }, []);

  useEffect(() => { loadData(); }, [loadData]);

  // ── Save Config ──
  const saveConfig = async () => {
    try {
      const res = await authFetch(`${API}/config`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(configForm),
      });
      const d = await res.json();
      if (d.ok) {
        showToast('✅ Đã lưu cấu hình TikTok', 'success');
        loadData();
      } else {
        showToast('❌ ' + (d.error || 'Lỗi lưu'), 'error');
      }
    } catch(e) {
      showToast('❌ ' + e.message, 'error');
    }
  };

  // ── OAuth Connect ──
  const connectOAuth = async () => {
    try {
      const res = await authFetch(`${API}/oauth`, { method: 'POST' });
      const d = await res.json();
      if (d.auth_url) {
        window.open(d.auth_url, '_blank');
        showToast('🔗 Đang mở TikTok OAuth...', 'info');
      } else {
        showToast('❌ Không thể khởi tạo OAuth', 'error');
      }
    } catch(e) {
      showToast('❌ ' + e.message, 'error');
    }
  };

  // ── Upload Video ──
  const uploadVideo = async () => {
    if (!videoForm.video_file) {
      showToast('⚠️ Chọn file video trước', 'error');
      return;
    }
    setUploading(true);
    setUploadProgress(0);

    const formData = new FormData();
    formData.append('video', videoForm.video_file);
    formData.append('title', videoForm.title);
    formData.append('description', videoForm.description);
    formData.append('privacy_level', videoForm.privacy_level);

    try {
      const xhr = new XMLHttpRequest();
      xhr.upload.addEventListener('progress', (e) => {
        if (e.lengthComputable) {
          setUploadProgress(Math.round((e.loaded / e.total) * 100));
        }
      });
      xhr.addEventListener('load', () => {
        setUploading(false);
        setUploadProgress(0);
        if (xhr.status === 200) {
          showToast('✅ Video đã upload thành công!', 'success');
          loadData();
          setVideoForm({ title: '', description: '', privacy_level: 'public', brand_category: '', for_fyp: true, video_file: null });
        } else {
          showToast('❌ Upload thất bại: ' + xhr.statusText, 'error');
        }
      });
      xhr.addEventListener('error', () => {
        setUploading(false);
        setUploadProgress(0);
        showToast('❌ Lỗi kết nối', 'error');
      });
      xhr.open('POST', `${API}/upload`);
      xhr.setRequestHeader('Authorization', `Bearer ${localStorage.getItem('bizclaw_token') || ''}`);
      xhr.send(formData);
    } catch(e) {
      setUploading(false);
      showToast('❌ ' + e.message, 'error');
    }
  };

  // ── Sync Orders ──
  const syncOrders = async () => {
    showToast('⏳ Đang đồng bộ đơn hàng...', 'info');
    try {
      const res = await authFetch(`${API}/sync`, { method: 'POST' });
      const d = await res.json();
      if (d.ok) {
        showToast(`✅ Đã sync ${d.count || 0} đơn hàng`, 'success');
        loadData();
      }
    } catch(e) {
      showToast('❌ ' + e.message, 'error');
    }
  };

  // ── Stats ──
  const stats = useMemo(() => {
    const totalOrders = orders.length;
    const pendingOrders = orders.filter(o => o.status === 'pending' || o.status === 'paid').length;
    const totalVideos = videos.length;
    const totalViews = videos.reduce((a, v) => a + (v.view_count || 0), 0);
    return { totalOrders, pendingOrders, totalVideos, totalViews };
  }, [orders, videos]);

  // ── Render ──
  const tabs = [
    { id: 'shop', icon: '🛒', label: 'Shop' },
    { id: 'video', icon: '🎬', label: 'Video' },
    { id: 'content', icon: '✍️', label: 'Content' },
    { id: 'settings', icon: '⚙️', label: 'Cấu hình' },
  ];

  return html`
    <div style="padding:20px">
      <!-- Header -->
      <div class="page-header">
        <div>
          <h1 style="margin:0">🎵 TikTok Integration</h1>
          <div style="color:var(--text2);font-size:13px;margin-top:4px">
            Quản lý Shop, Video, Content trên TikTok
          </div>
        </div>
        <div style="display:flex;gap:8px">
          <button class="btn btn-outline" onClick=${syncOrders}>🔄 Sync Orders</button>
        </div>
      </div>

      <!-- Stats -->
      <div class="stats" style="margin-bottom:20px">
        <${StatsCard} icon="🛒" label="Tổng đơn" value=${stats.totalOrders} color="blue" />
        <${StatsCard} icon="⏳" label="Đơn chờ" value=${stats.pendingOrders} color="orange" />
        <${StatsCard} icon="🎬" label="Videos" value=${stats.totalVideos} color="purple" />
        <${StatsCard} icon="👁" label="Lượt xem" value=${stats.totalViews.toLocaleString()} color="green" />
      </div>

      <!-- Tabs -->
      <div style="display:flex;gap:4px;margin-bottom:20px;border-bottom:1px solid var(--border);padding-bottom:12px">
        ${tabs.map(tab => html`
          <button 
            onClick=${() => setActiveTab(tab.id)}
            style="padding:8px 16px;border-radius:8px 8px 0 0;border:none;cursor:pointer;font-weight:600;font-size:13px;
              background:${activeTab === tab.id ? 'var(--accent)' : 'var(--surface2)'};
              color:${activeTab === tab.id ? '#fff' : 'var(--text2)'};
              transition:all .2s"
          >
            ${tab.icon} ${tab.label}
          </button>
        `)}
      </div>

      ${loading && html`<div style="text-align:center;padding:40px">⏳ Đang tải...</div>`}

      ${!loading && activeTab === 'shop' && html`
        <div style="display:grid;grid-template-columns:1fr 1fr;gap:20px">
          <!-- Orders -->
          <div class="card">
            <h3 style="margin-top:0">🛒 Đơn hàng gần đây</h3>
            ${orders.length === 0 ? html`
              <div style="text-align:center;padding:20px;color:var(--text2)">
                Chưa có đơn hàng nào
              </div>
            ` : html`
              <table style="width:100%;font-size:13px">
                <thead>
                  <tr style="border-bottom:1px solid var(--border)">
                    <th style="text-align:left;padding:8px 0">Mã đơn</th>
                    <th style="text-align:left;padding:8px 0">Sản phẩm</th>
                    <th style="text-align:right;padding:8px 0">Giá trị</th>
                    <th style="text-align:center;padding:8px 0">Status</th>
                  </tr>
                </thead>
                <tbody>
                  ${orders.slice(0, 10).map(o => html`
                    <tr key=${o.order_id} style="border-bottom:1px solid var(--border2)">
                      <td style="padding:8px 0;font-family:monospace;font-size:12px">${o.order_id}</td>
                      <td style="padding:8px 0">${o.items?.length || 0} sản phẩm</td>
                      <td style="padding:8px 0;text-align:right">${(o.total_amount || 0).toLocaleString()}đ</td>
                      <td style="padding:8px 0;text-align:center">
                        <span class="badge ${o.status === 'delivered' ? 'badge-green' : o.status === 'shipped' ? 'badge-blue' : 'badge-orange'}">
                          ${o.status}
                        </span>
                      </td>
                    </tr>
                  `)}
                </tbody>
              </table>
            `}
          </div>

          <!-- Products -->
          <div class="card">
            <h3 style="margin-top:0">📦 Sản phẩm</h3>
            ${products.length === 0 ? html`
              <div style="text-align:center;padding:20px;color:var(--text2)">
                Chưa có sản phẩm nào
              </div>
            ` : html`
              <div style="display:grid;grid-template-columns:repeat(2,1fr);gap:12px">
                ${products.slice(0, 6).map(p => html`
                  <div key=${p.product_id} style="padding:12px;background:var(--bg2);border-radius:8px">
                    <div style="font-weight:600;font-size:13px;margin-bottom:4px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis">${p.name}</div>
                    <div style="font-size:12px;color:var(--text2)">Tồn kho: ${p.stock || 0}</div>
                    <div style="font-size:12px;color:var(--accent)">${(p.price || 0).toLocaleString()}đ</div>
                  </div>
                `)}
              </div>
            `}
          </div>
        </div>
      `}

      ${!loading && activeTab === 'video' && html`
        <div style="display:grid;grid-template-columns:1fr 1fr;gap:20px">
          <!-- Upload Form -->
          <div class="card">
            <h3 style="margin-top:0">📤 Upload Video mới</h3>
            
            <label style="display:block;margin-bottom:12px">
              <div style="font-size:13px;font-weight:500;margin-bottom:4px">File Video</div>
              <input 
                type="file" 
                accept="video/*"
                onChange=${e => setVideoForm(f => ({...f, video_file: e.target.files[0]}))}
                style="width:100%;padding:8px;background:var(--bg2);border:1px solid var(--border);border-radius:6px"
              />
            </label>

            <label style="display:block;margin-bottom:12px">
              <div style="font-size:13px;font-weight:500;margin-bottom:4px">Tiêu đề</div>
              <input 
                type="text"
                value=${videoForm.title}
                onInput=${e => setVideoForm(f => ({...f, title: e.target.value}))}
                placeholder="VD: Sản phẩm mới - Giảm 50%"
                style=${inp}
              />
            </label>

            <label style="display:block;margin-bottom:12px">
              <div style="font-size:13px;font-weight:500;margin-bottom:4px">Mô tả</div>
              <textarea 
                value=${videoForm.description}
                onInput=${e => setVideoForm(f => ({...f, description: e.target.value}))}
                placeholder="Mô tả video..."
                rows="3"
                style=${inp + ';resize:vertical'}
              ></textarea>
            </label>

            <label style="display:block;margin-bottom:12px">
              <div style="font-size:13px;font-weight:500;margin-bottom:4px">Quyền riêng tư</div>
              <select 
                value=${videoForm.privacy_level}
                onChange=${e => setVideoForm(f => ({...f, privacy_level: e.target.value}))}
                style=${inp}
              >
                <option value="public">🌍 Công khai</option>
                <option value="private">🔒 Riêng tư</option>
                <option value="friends">👥 Bạn bè</option>
              </select>
            </label>

            ${uploading && html`
              <div style="margin-bottom:12px">
                <div style="font-size:13px;margin-bottom:6px">📤 Đang upload... ${uploadProgress}%</div>
                <div style="height:6px;background:var(--bg3);border-radius:3px;overflow:hidden">
                  <div style="height:100%;width:${uploadProgress}%;background:var(--accent);transition:width .3s"></div>
                </div>
              </div>
            `}

            <button 
              class="btn" 
              style="width:100%;background:var(--accent);color:#fff"
              onClick=${uploadVideo}
              disabled=${uploading}
            >
              ${uploading ? '⏳ Đang upload...' : '📤 Upload Video'}
            </button>
          </div>

          <!-- Recent Videos -->
          <div class="card">
            <h3 style="margin-top:0">🎬 Videos đã đăng</h3>
            ${videos.length === 0 ? html`
              <div style="text-align:center;padding:40px;color:var(--text2)">
                Chưa có video nào
              </div>
            ` : html`
              <div style="display:flex;flex-direction:column;gap:12px">
                ${videos.slice(0, 5).map(v => html`
                  <div key=${v.video_id} style="display:flex;gap:12px;padding:12px;background:var(--bg2);border-radius:8px">
                    <div style="width:80px;height:80px;background:var(--bg3);border-radius:6px;display:flex;align-items:center;justify-content:center;font-size:24px">
                      🎵
                    </div>
                    <div style="flex:1">
                      <div style="font-weight:600;font-size:13px;margin-bottom:4px">${v.title || 'Video không tiêu đề'}</div>
                      <div style="font-size:12px;color:var(--text2);margin-bottom:4px">${v.create_time ? new Date(v.create_time * 1000).toLocaleDateString('vi-VN') : ''}</div>
                      <div style="display:flex;gap:12px;font-size:12px">
                        <span>👁 ${(v.view_count || 0).toLocaleString()}</span>
                        <span>❤️ ${(v.like_count || 0).toLocaleString()}</span>
                        <span>💬 ${(v.comment_count || 0).toLocaleString()}</span>
                      </div>
                    </div>
                  </div>
                `)}
              </div>
            `}
          </div>
        </div>
      `}

      ${!loading && activeTab === 'settings' && html`
        <div class="card" style="max-width:600px">
          <h3 style="margin-top:0">⚙️ Cấu hình TikTok</h3>
          
          <div style="display:flex;flex-direction:column;gap:16px">
            <div style="padding:12px;background:var(--accent-glow, rgba(99,102,241,0.1));border-radius:8px;border:1px solid var(--accent)">
              <div style="font-size:13px;font-weight:600;margin-bottom:4px">🔗 Kết nối OAuth</div>
              <div style="font-size:12px;color:var(--text2);margin-bottom:8px">
                Sử dụng OAuth để lấy Access Token tự động
              </div>
              <button class="btn" style="background:var(--accent);color:#fff" onClick=${connectOAuth}>
                🔗 Kết nối TikTok
              </button>
            </div>

            <hr style="border:none;border-top:1px solid var(--border);margin:8px 0" />
            <div style="font-size:13px;font-weight:600;margin-bottom:8px">Hoặc nhập thủ công</div>

            <label style="display:block">
              <div style="font-size:13px;font-weight:500;margin-bottom:4px">App ID</div>
              <input 
                type="text"
                value=${configForm.app_id}
                onInput=${e => setConfigForm(f => ({...f, app_id: e.target.value}))}
                placeholder="123456789"
                style=${inp}
              />
            </label>

            <label style="display:block">
              <div style="font-size:13px;font-weight:500;margin-bottom:4px">App Secret</div>
              <input 
                type="password"
                value=${configForm.app_secret}
                onInput=${e => setConfigForm(f => ({...f, app_secret: e.target.value}))}
                placeholder="••••••••••••"
                style=${inp}
              />
            </label>

            <label style="display:block">
              <div style="font-size:13px;font-weight:500;margin-bottom:4px">Access Token</div>
              <input 
                type="password"
                value=${configForm.access_token}
                onInput=${e => setConfigForm(f => ({...f, access_token: e.target.value}))}
                placeholder="Access token từ TikTok"
                style=${inp}
              />
            </label>

            <label style="display:block">
              <div style="font-size:13px;font-weight:500;margin-bottom:4px">Shop ID</div>
              <input 
                type="text"
                value=${configForm.shop_id}
                onInput=${e => setConfigForm(f => ({...f, shop_id: e.target.value}))}
                placeholder="789123456"
                style=${inp}
              />
            </label>

            <button 
              class="btn" 
              style="background:var(--accent);color:#fff"
              onClick=${saveConfig}
            >
              💾 Lưu cấu hình
            </button>
          </div>
        </div>
      `}

      ${!loading && activeTab === 'content' && html`
        <div class="card">
          <h3 style="margin-top:0">✍️ AI Content Generator</h3>
          <div style="padding:20px;text-align:center;color:var(--text2)">
            Tính năng đang được phát triển...
            <div style="margin-top:12px">
              <button class="btn btn-outline" onClick=${() => showToast('🔜 Sắp ra mắt!', 'info')}>
                📝 Tạo Content bằng AI
              </button>
            </div>
          </div>
        </div>
      `}
    </div>
  `;
}

export { TikTokPage };
