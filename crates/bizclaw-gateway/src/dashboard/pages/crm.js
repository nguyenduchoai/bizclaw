// CRM Page - OmniChannel Customer 360 Dashboard

const { useState, useEffect, useMemo } = preactHooks;

function CRM() {
  const [contacts, setContacts] = useState([]);
  const [selectedContact, setSelectedContact] = useState(null);
  const [activeTab, setActiveTab] = useState('all');
  const [searchQuery, setSearchQuery] = useState('');

  useEffect(() => {
    loadContacts();
  }, []);

  async function loadContacts() {
    const res = await api('/api/crm/contacts');
    if (res.ok) {
      const data = await res.json();
      setContacts(data);
    }
  }

  const filtered = useMemo(() => {
    return contacts.filter(c => {
      if (activeTab === 'all') return true;
      if (activeTab === 'unread') return c.unreadCount > 0;
      return c.pipeline === activeTab;
    }).filter(c => 
      c.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      c.channels.some(ch => ch.phone?.includes(searchQuery))
    );
  }, [contacts, activeTab, searchQuery]);

  return html`
    <div class="page">
      <div class="page-header">
        <h1>📊 OmniChannel CRM</h1>
        <div class="header-actions">
          <input 
            type="text" 
            placeholder="Tìm kiếm khách hàng..." 
            value=${searchQuery}
            onInput=${e => setSearchQuery(e.target.value)}
          />
          <button onClick=${() => setActiveTab('all')}>Tất cả</button>
          <button onClick=${() => setActiveTab('new')}>🆕 Mới</button>
          <button onClick=${() => setActiveTab('contacted')}>📞 Đã liên hệ</button>
          <button onClick=${() => setActiveTab('interested')}>🤔 Quan tâm</button>
          <button onClick=${() => setActiveTab('converted')}>✅ Chuyển đổi</button>
        </div>
      </div>

      <div class="crm-layout">
        <div class="contact-list">
          ${filtered.map(contact => html`
            <div 
              class="contact-card ${selectedContact?.id === contact.id ? 'selected' : ''}"
              onClick=${() => setSelectedContact(contact)}
            >
              <div class="contact-avatar">
                ${contact.avatar || '👤'}
              </div>
              <div class="contact-info">
                <div class="contact-name">${contact.primaryName || contact.name}</div>
                <div class="contact-channels">
                  ${contact.channels?.map(ch => html`<span class="channel-badge">${ch.channel}</span>`)}
                </div>
              </div>
              <div class="contact-status ${contact.pipeline}>${contact.pipelineLabel}</div>
            </div>
          `)}
        </div>

        ${selectedContact && html`
          <div class="contact-detail">
            <div class="detail-header">
              <h2>${selectedContact.primaryName || selectedContact.name}</h2>
              <span class="channel-badge">${selectedContact.channels?.[0]?.channel}</span>
            </div>
            
            <div class="detail-section">
              <h3>Thông tin</h3>
              <p><strong>Điện thoại:</strong> ${selectedContact.channels?.map(c => c.phone?.join(', '))}</p>
              <p><strong>Email:</strong> ${selectedContact.channels?.map(c => c.email?.join(', '))}</p>
              <p><strong>Địa chỉ:</strong> ${selectedContact.address || 'N/A'}</p>
            </div>

            <div class="detail-section">
              <h3>Lịch sử tương tác</h3>
              ${selectedContact.interactions?.map(i => html`
                <div class="interaction-item">
                  <span class="channel-badge">${i.channel}</span>
                  <span>${i.content}</span>
                  <small>${new Date(i.createdAt).toLocaleString('vi')}</small>
                </div>
              `)}
            </div>

            <div class="detail-section">
              <h3>Giao dịch</h3>
              ${selectedContact.transactions?.map(t => html`
                <div class="transaction-item">
                  <span>${t.orderId}</span>
                  <span>${t.totalAmount?.toLocaleString()}đ</span>
                  <span class="badge ${t.status}">${t.status}</span>
                </div>
              `)}
            </div>

            <div class="detail-section">
              <h3>Ticket hỗ trợ</h3>
              ${selectedContact.tickets?.map(ticket => html`
                <div class="ticket-item ${ticket.status}">
                  <strong>${ticket.subject}</strong>
                  <span>${ticket.status}</span>
                </div>
              `)}
            </div>
          </div>
        `}
      </div>
    </div>
  `;
}

window.CRM = CRM;
