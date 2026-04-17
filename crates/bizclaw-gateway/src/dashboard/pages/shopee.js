// ═══════════════════════════════════════════════════════════════
// BizClaw — Shopee E-commerce Dashboard
// Phase 1: Products, Orders, Inventory Sync
// ═══════════════════════════════════════════════════════════════
const { h, html, useState, useEffect, useContext, useCallback, useMemo } = window;
const { authFetch, t, StatsCard } = window;

const API = '/api/v1/shopee';

function ShopeePage({ lang }) {
  const { showToast } = useContext(AppContext);
  
  const [activeTab, setActiveTab] = useState('products'); // products | orders | inventory | settings
  const [loading, setLoading] = useState(true);
  const [config, setConfig] = useState(null);
  const [orders, setOrders] = useState([]);
  const [products, setProducts] = useState([]);
  const [inventory, setInventory] = useState([]);
  const [syncing, setSyncing] = useState(false);

  const [configForm, setConfigForm] = useState({
    partner_id: '',
    shop_id: '',
    api_key: '',
    secret_key: '',
  });

  const [filter, setFilter] = useState('all');
  const [search, setSearch] = useState('');

  const inp = 'width:100%;padding:8px;margin-top:4px;background:var(--bg2);border:1px solid var(--border);border-radius:6px;color:var(--text);font-size:13px';

  const loadData = useCallback(async () => {
    setLoading(true);
    try {
      const [configRes, ordersRes, productsRes, invRes] = await Promise.all([
        authFetch(`${API}/config`),
        authFetch(`${API}/orders`),
        authFetch(`${API}/products`),
        authFetch(`${API}/inventory`),
      ]);

      if (configRes.ok) {
        const d = await configRes.json();
        setConfig(d);
        setConfigForm({
          partner_id: d.partner_id || '',
          shop_id: d.shop_id || '',
          api_key: d.api_key || '',
          secret_key: d.secret_key || '',
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
      if (invRes.ok) {
        const d = await invRes.json();
        setInventory(d.items || []);
      }
    } catch(e) {
      console.error('Shopee load error:', e);
    }
    setLoading(false);
  }, []);

  useEffect(() => { loadData(); }, [loadData]);

  const saveConfig = async () => {
    try {
      const res = await authFetch(`${API}/config`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(configForm),
      });
      const d = await res.json();
      if (d.ok) {
        showToast('✅ Đã lưu cấu hình Shopee', 'success');
        loadData();
      } else {
        showToast('❌ ' + (d.error || 'Lỗi lưu'), 'error');
      }
    } catch(e) {
      showToast('❌ ' + e.message, 'error');
    }
  };

  const syncAll = async () => {
    setSyncing(true);
    showToast('⏳ Đang đồng bộ...', 'info');
    try {
      const res = await authFetch(`${API}/sync`, { method: 'POST' });
      const d = await res.json();
      showToast(d.ok ? `✅ Đã sync ${d.count || 0} items` : '❌ Sync thất bại', d.ok ? 'success' : 'error');
      loadData();
    } catch(e) {
      showToast('❌ ' + e.message, 'error');
    }
    setSyncing(false);
  };

  const updateStock = async (product_id, new_stock) => {
    try {
      const res = await authFetch(`${API}/inventory`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ product_id, stock: new_stock }),
      });
      const d = await res.json();
      if (d.ok) {
        showToast('✅ Đã cập nhật tồn kho', 'success');
        loadData();
      }
    } catch(e) {
      showToast('❌ ' + e.message, 'error');
    }
  };

  const stats = useMemo(() => {
    const totalProducts = products.length;
    const lowStock = inventory.filter(i => i.stock < 10).length;
    const totalOrders = orders.length;
    const pendingOrders = orders.filter(o => o.status === 'pending' || o.status === 'paid').length;
    const revenue = orders.reduce((a, o) => a + (o.revenue || 0), 0);
    return { totalProducts, lowStock, totalOrders, pendingOrders, revenue };
  }, [products, inventory, orders]);

  const tabs = [
    { id: 'products', icon: '📦', label: 'Sản phẩm' },
    { id: 'orders', icon: '🛒', label: 'Đơn hàng' },
    { id: 'inventory', icon: '📊', label: 'Tồn kho' },
    { id: 'settings', icon: '⚙️', label: 'Cấu hình' },
  ];

  const filteredProducts = useMemo(() => {
    let list = products;
    if (filter === 'lowstock') list = list.filter(p => p.stock < 10);
    if (filter === 'active') list = list.filter(p => p.status === 'active');
    if (filter === 'inactive') list = list.filter(p => p.status === 'inactive');
    if (search) {
      const q = search.toLowerCase();
      list = list.filter(p => 
        p.name?.toLowerCase().includes(q) || 
        p.sku?.toLowerCase().includes(q)
      );
    }
    return list;
  }, [products, filter, search]);

  const filteredOrders = useMemo(() => {
    let list = orders;
    if (filter !== 'all') {
      list = list.filter(o => o.status === filter);
    }
    if (search) {
      const q = search.toLowerCase();
      list = list.filter(o => 
        o.order_id?.toLowerCase().includes(q) ||
        o.buyer_name?.toLowerCase().includes(q)
      );
    }
    return list;
  }, [orders, filter, search]);

  return html`
    <div style="padding:20px">
      <!-- Header -->
      <div class="page-header">
        <div>
          <h1 style="margin:0">🛒 Shopee Integration</h1>
          <div style="color:var(--text2);font-size:13px;margin-top:4px">
            Quản lý sản phẩm, đơn hàng, tồn kho Shopee
          </div>
        </div>
        <div style="display:flex;gap:8px">
          <button class="btn btn-outline" onClick=${syncAll} disabled=${syncing}>
            ${syncing ? '⏳ Đang sync...' : '🔄 Sync All'}
          </button>
        </div>
      </div>

      <!-- Stats -->
      <div class="stats" style="margin-bottom:20px">
        <${StatsCard} icon="📦" label="Sản phẩm" value=${stats.totalProducts} color="blue" />
        <${StatsCard} icon="⚠️" label="Sắp hết" value=${stats.lowStock} color="orange" />
        <${StatsCard} icon="🛒" label="Đơn hàng" value=${stats.totalOrders} color="purple" />
        <${StatsCard} icon="⏳" label="Chờ xử lý" value=${stats.pendingOrders} color="orange" />
        <${StatsCard} icon="💰" label="Doanh thu" value=${(stats.revenue / 1000000).toFixed(1) + 'Mđ'} color="green" />
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

      ${!loading && activeTab === 'products' && html`
        <!-- Filters -->
        <div style="display:flex;gap:12px;margin-bottom:16px">
          <input 
            type="text"
            placeholder="🔍 Tìm sản phẩm..."
            value=${search}
            onInput=${e => setSearch(e.target.value)}
            style=${inp + ';max-width:300px'}
          />
          <select 
            value=${filter}
            onChange=${e => setFilter(e.target.value)}
            style=${inp + ';width:150px'}
          >
            <option value="all">Tất cả</option>
            <option value="active">Đang bán</option>
            <option value="inactive">Ngừng bán</option>
            <option value="lowstock">Sắp hết hàng</option>
          </select>
        </div>

        <!-- Products Grid -->
        <div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(280px,1fr));gap:16px">
          ${filteredProducts.length === 0 ? html`
            <div style="grid-column:1/-1;text-align:center;padding:40px;color:var(--text2)">
              Không có sản phẩm nào
            </div>
          ` : filteredProducts.map(p => html`
            <div key=${p.product_id || p.id} class="card" style="padding:16px">
              <div style="display:flex;gap:12px">
                <div style="width:64px;height:64px;background:var(--bg3);border-radius:8px;display:flex;align-items:center;justify-content:center;font-size:24px">
                  📦
                </div>
                <div style="flex:1;min-width:0">
                  <div style="font-weight:600;font-size:14px;margin-bottom:4px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis">
                    ${p.name}
                  </div>
                  <div style="font-size:12px;color:var(--text2);margin-bottom:4px">
                    SKU: ${p.sku || '-'}
                  </div>
                  <div style="display:flex;gap:8px;font-size:12px">
                    <span style="color:${p.stock < 10 ? 'var(--red)' : 'var(--green)'}">
                      📦 ${p.stock || 0}
                    </span>
                    <span style="color:var(--accent)">
                      💰 ${(p.price || 0).toLocaleString()}đ
                    </span>
                  </div>
                </div>
              </div>
              <div style="margin-top:12px;display:flex;justify-content:space-between;align-items:center">
                <span class="badge ${p.status === 'active' ? 'badge-green' : 'badge-outline'}">
                  ${p.status === 'active' ? '✅ Đang bán' : '⏸ Ngừng'}
                </span>
              </div>
            </div>
          `)}
        </div>
      `}

      ${!loading && activeTab === 'orders' && html`
        <!-- Filters -->
        <div style="display:flex;gap:12px;margin-bottom:16px">
          <input 
            type="text"
            placeholder="🔍 Tìm đơn hàng..."
            value=${search}
            onInput=${e => setSearch(e.target.value)}
            style=${inp + ';max-width:300px'}
          />
          <select 
            value=${filter}
            onChange=${e => setFilter(e.target.value)}
            style=${inp + ';width:180px'}
          >
            <option value="all">Tất cả</option>
            <option value="pending">⏳ Chờ xử lý</option>
            <option value="paid">✅ Đã thanh toán</option>
            <option value="shipped">🚚 Đang giao</option>
            <option value="delivered">📦 Đã giao</option>
            <option value="cancelled">❌ Đã hủy</option>
          </select>
        </div>

        <!-- Orders Table -->
        <div class="card" style="overflow-x:auto">
          <table style="width:100%;font-size:13px;min-width:800px">
            <thead>
              <tr style="border-bottom:2px solid var(--border)">
                <th style="text-align:left;padding:12px 8px">Mã đơn</th>
                <th style="text-align:left;padding:12px 8px">Khách hàng</th>
                <th style="text-align:right;padding:12px 8px">Tổng giá</th>
                <th style="text-align:center;padding:12px 8px">Status</th>
                <th style="text-align:center;padding:12px 8px">Ngày đặt</th>
                <th style="text-align:center;padding:12px 8px">Actions</th>
              </tr>
            </thead>
            <tbody>
              ${filteredOrders.length === 0 ? html`
                <tr>
                  <td colspan="6" style="text-align:center;padding:40px;color:var(--text2)">
                    Không có đơn hàng nào
                  </td>
                </tr>
              ` : filteredOrders.map(o => html`
                <tr key=${o.order_id} style="border-bottom:1px solid var(--border2)">
                  <td style="padding:12px 8px;font-family:monospace;font-size:12px">${o.order_id}</td>
                  <td style="padding:12px 8px">${o.buyer_name || o.buyer_username || 'Khách'}</td>
                  <td style="padding:12px 8px;text-align:right;font-weight:600">${(o.total_amount || o.revenue || 0).toLocaleString()}đ</td>
                  <td style="padding:12px 8px;text-align:center">
                    <span class="badge ${o.status === 'delivered' ? 'badge-green' : o.status === 'shipped' ? 'badge-blue' : 'badge-orange'}">
                      ${o.status}
                    </span>
                  </td>
                  <td style="padding:12px 8px;text-align:center;font-size:12px;color:var(--text2)">
                    ${o.create_time ? new Date(o.create_time * 1000).toLocaleDateString('vi-VN') : '-'}
                  </td>
                  <td style="padding:12px 8px;text-align:center">
                    <button class="btn btn-outline btn-sm" onClick=${() => showToast('🔜 Chi tiết đơn hàng...', 'info')}>
                      👁
                    </button>
                  </td>
                </tr>
              `)}
            </tbody>
          </table>
        </div>
      `}

      ${!loading && activeTab === 'inventory' && html`
        <div class="card" style="overflow-x:auto">
          <h3 style="margin-top:0">📊 Tồn kho Shopee</h3>
          <table style="width:100%;font-size:13px">
            <thead>
              <tr style="border-bottom:2px solid var(--border)">
                <th style="text-align:left;padding:12px 8px">Sản phẩm</th>
                <th style="text-align:center;padding:12px 8px">Tồn kho</th>
                <th style="text-align:center;padding:12px 8px">Giá bán</th>
                <th style="text-align:center;padding:12px 8px">Actions</th>
              </tr>
            </thead>
            <tbody>
              ${inventory.length === 0 ? html`
                <tr>
                  <td colspan="4" style="text-align:center;padding:40px;color:var(--text2)">
                    Không có dữ liệu tồn kho
                  </td>
                </tr>
              ` : inventory.map(item => html`
                <tr key=${item.product_id} style="border-bottom:1px solid var(--border2)">
                  <td style="padding:12px 8px">
                    <div style="font-weight:500">${item.name}</div>
                    <div style="font-size:12px;color:var(--text2)">${item.sku}</div>
                  </td>
                  <td style="padding:12px 8px;text-align:center">
                    <span style="color:${item.stock < 10 ? 'var(--red)' : 'inherit'}">
                      ${item.stock || 0}
                    </span>
                  </td>
                  <td style="padding:12px 8px;text-align:center">${(item.price || 0).toLocaleString()}đ</td>
                  <td style="padding:12px 8px;text-align:center">
                    <button 
                      class="btn btn-outline btn-sm"
                      onClick=${() => {
                        const newStock = prompt('Nhập số lượng mới:', item.stock);
                        if (newStock !== null) {
                          updateStock(item.product_id, parseInt(newStock));
                        }
                      }}
                    >
                      ✏️ Cập nhật
                    </button>
                  </td>
                </tr>
              `)}
            </tbody>
          </table>
        </div>
      `}

      ${!loading && activeTab === 'settings' && html`
        <div class="card" style="max-width:600px">
          <h3 style="margin-top:0">⚙️ Cấu hình Shopee</h3>
          
          <div style="display:flex;flex-direction:column;gap:16px">
            <div style="padding:12px;background:var(--accent-glow, rgba(99,102,241,0.1));border-radius:8px">
              <div style="font-size:13px">
                Để lấy thông tin kết nối, đăng ký tài khoản Shopee Partner tại 
                <a href="https://partner.shopeemobile.com" target="_blank" style="color:var(--accent)">partner.shopeemobile.com</a>
              </div>
            </div>

            <label style="display:block">
              <div style="font-size:13px;font-weight:500;margin-bottom:4px">Partner ID</div>
              <input 
                type="text"
                value=${configForm.partner_id}
                onInput=${e => setConfigForm(f => ({...f, partner_id: e.target.value}))}
                placeholder="1234567"
                style=${inp}
              />
            </label>

            <label style="display:block">
              <div style="font-size:13px;font-weight:500;margin-bottom:4px">Shop ID</div>
              <input 
                type="text"
                value=${configForm.shop_id}
                onInput=${e => setConfigForm(f => ({...f, shop_id: e.target.value}))}
                placeholder="1234567890"
                style=${inp}
              />
            </label>

            <label style="display:block">
              <div style="font-size:13px;font-weight:500;margin-bottom:4px">API Key</div>
              <input 
                type="password"
                value=${configForm.api_key}
                onInput=${e => setConfigForm(f => ({...f, api_key: e.target.value}))}
                placeholder="••••••••••••"
                style=${inp}
              />
            </label>

            <label style="display:block">
              <div style="font-size:13px;font-weight:500;margin-bottom:4px">Secret Key</div>
              <input 
                type="password"
                value=${configForm.secret_key}
                onInput=${e => setConfigForm(f => ({...f, secret_key: e.target.value}))}
                placeholder="••••••••••••"
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
    </div>
  `;
}

export { ShopeePage };
