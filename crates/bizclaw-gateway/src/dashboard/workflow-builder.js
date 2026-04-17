// Visual Workflow Builder - Drag & Drop Logic

const WORKFLOW_STEP_TYPES = {
  trigger: { icon: '⚡', label: 'Trigger', color: '#f59e0b' },
  action: { icon: '🎯', label: 'Action', color: '#3b82f6' },
  condition: { icon: '🔀', label: 'If/Else', color: '#10b981' },
  loop: { icon: '🔁', label: 'Loop', color: '#8b5cf6' },
  delay: { icon: '⏰', label: 'Delay', color: '#64748b' },
  agent: { icon: '🤖', label: 'AI Agent', color: '#ec4899' },
  notify: { icon: '📢', label: 'Notify', color: '#06b6d4' },
  transform: { icon: '✨', label: 'Transform', color: '#f59e0b' },
  query: { icon: '🗄️', label: 'SQL Query', color: '#6366f1' },
  rag: { icon: '📚', label: 'RAG Search', color: '#14b8a6' },
};

const STEP_CONFIG_FIELDS = {
  trigger: [
    { key: 'channel', label: 'Kênh kích hoạt', type: 'select', options: ['zalo', 'messenger', 'telegram', 'webhook', 'schedule'] },
  ],
  action: [
    { key: 'agent', label: 'Agent', type: 'text', placeholder: 'Tên agent' },
    { key: 'prompt', label: 'Prompt', type: 'textarea', placeholder: 'Hướng dẫn cho action...' },
  ],
  condition: [
    { key: 'field', label: 'Trường so sánh', type: 'text', placeholder: 'field.name' },
    { key: 'operator', label: 'Toán tử', type: 'select', options: ['equals', 'not_equals', 'contains', 'greater_than', 'less_than'] },
    { key: 'value', label: 'Giá trị', type: 'text', placeholder: 'Giá trị so sánh' },
  ],
  loop: [
    { key: 'maxIterations', label: 'Số lần tối đa', type: 'number', placeholder: '10' },
    { key: 'condition', label: 'Điều kiện dừng', type: 'text', placeholder: 'field.value > 0' },
  ],
  delay: [
    { key: 'seconds', label: 'Thời gian (giây)', type: 'number', placeholder: '5' },
  ],
  agent: [
    { key: 'agent', label: 'AI Agent', type: 'select', options: ['sales-agent', 'support-agent', 'writer-agent', 'analyst-agent'] },
    { key: 'prompt', label: 'System Prompt', type: 'textarea', placeholder: 'Hướng dẫn cho agent...' },
  ],
  notify: [
    { key: 'channel', label: 'Kênh gửi', type: 'select', options: ['zalo', 'email', 'telegram', 'sms'] },
    { key: 'template', label: 'Template', type: 'textarea', placeholder: 'Nội dung thông báo...' },
  ],
  transform: [
    { key: 'template', label: 'Template', type: 'textarea', placeholder: '{{field1}} - {{field2}}' },
  ],
  query: [
    { key: 'sql', label: 'SQL Query', type: 'textarea', placeholder: 'SELECT * FROM...' },
  ],
  rag: [
    { key: 'knowledgeBase', label: 'Knowledge Base', type: 'text', placeholder: 'Tên knowledge base' },
    { key: 'query', label: 'Query', type: 'text', placeholder: 'Câu hỏi tìm kiếm...' },
  ],
};

class WorkflowBuilder {
  constructor() {
    this.steps = [];
    this.connections = [];
    this.selectedStep = null;
    this.nextId = 1;
    this.workflowId = this.generateId();
    this.draggedPaletteItem = null;
    this.connectionMode = null;
    this.pendingConnection = null;

    this.init();
  }

  generateId() {
    return 'wf_' + Date.now().toString(36) + Math.random().toString(36).substr(2, 5);
  }

  init() {
    this.setupPaletteDrag();
    this.setupCanvasDrop();
    this.loadFromStorage();
    this.render();
    this.updateInfo();
  }

  setupPaletteDrag() {
    const items = document.querySelectorAll('.step-item[draggable="true"]');
    items.forEach(item => {
      item.addEventListener('dragstart', (e) => {
        this.draggedPaletteItem = {
          type: item.dataset.type,
          config: JSON.parse(item.dataset.config || '{}'),
        };
        item.classList.add('dragging');
        e.dataTransfer.effectAllowed = 'copy';
      });

      item.addEventListener('dragend', () => {
        item.classList.remove('dragging');
        this.draggedPaletteItem = null;
      });
    });
  }

  setupCanvasDrop() {
    const canvas = document.getElementById('workflowCanvas');
    const container = document.getElementById('stepsContainer');
    const startPoint = document.getElementById('workflowStart');

    canvas.addEventListener('dragover', (e) => {
      e.preventDefault();
      e.dataTransfer.dropEffect = 'copy';
    });

    canvas.addEventListener('drop', (e) => {
      e.preventDefault();
      if (this.draggedPaletteItem) {
        const rect = container.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;

        this.addStep(this.draggedPaletteItem.type, this.draggedPaletteItem.config, y);
        this.draggedPaletteItem = null;
      }
    });

    startPoint.querySelector('.connector-point').addEventListener('click', (e) => {
      e.stopPropagation();
      this.startConnection('start');
    });
  }

  addStep(type, config = {}, position = null) {
    const id = this.nextId++;
    const stepType = WORKFLOW_STEP_TYPES[type] || WORKFLOW_STEP_TYPES.action;
    
    const step = {
      id,
      type,
      name: stepType.label + ' ' + id,
      config: { ...config },
      position: position || (this.steps.length * 80 + 100),
    };

    this.steps.push(step);
    this.render();
    this.saveToStorage();
    this.updateInfo();
    this.selectStep(id);

    return step;
  }

  selectStep(id) {
    this.selectedStep = id;
    document.querySelectorAll('.step-block').forEach(el => {
      el.classList.toggle('selected', parseInt(el.dataset.id) === id);
    });
    this.updatePropertiesPanel();
  }

  updatePropertiesPanel() {
    const emptyPanel = document.getElementById('stepProperties');
    const formPanel = document.getElementById('stepPropertiesForm');

    if (!this.selectedStep) {
      emptyPanel.style.display = 'flex';
      formPanel.style.display = 'none';
      return;
    }

    const step = this.steps.find(s => s.id === this.selectedStep);
    if (!step) return;

    emptyPanel.style.display = 'none';
    formPanel.style.display = 'flex';

    document.getElementById('propName').value = step.name;
    document.getElementById('propType').textContent = WORKFLOW_STEP_TYPES[step.type]?.label || step.type;
    document.getElementById('propId').textContent = '#' + step.id;

    this.renderConfigFields(step);
  }

  renderConfigFields(step) {
    const container = document.getElementById('configFields');
    const fields = STEP_CONFIG_FIELDS[step.type] || [];

    container.innerHTML = '';

    if (fields.length === 0) {
      container.innerHTML = '<div class="form-group"><label>Cấu hình (JSON)</label><textarea id="propConfigJson" class="form-control" rows="4">' + JSON.stringify(step.config, null, 2) + '</textarea></div>';
      document.getElementById('propConfigJson').addEventListener('change', (e) => {
        try {
          step.config = JSON.parse(e.target.value);
          this.saveToStorage();
        } catch (err) {
          this.showToast('JSON không hợp lệ', 'error');
        }
      });
      return;
    }

    fields.forEach(field => {
      const div = document.createElement('div');
      div.className = 'form-group';
      div.innerHTML = `
        <label>${field.label}</label>
        ${field.type === 'select' 
          ? `<select class="form-control" data-key="${field.key}">
              ${field.options.map(opt => `<option value="${opt}" ${step.config[field.key] === opt ? 'selected' : ''}>${opt}</option>`).join('')}
             </select>`
          : field.type === 'textarea'
          ? `<textarea class="form-control" data-key="${field.key}" placeholder="${field.placeholder || ''}">${step.config[field.key] || ''}</textarea>`
          : `<input type="${field.type}" class="form-control" data-key="${field.key}" value="${step.config[field.key] || ''}" placeholder="${field.placeholder || ''}">`
        }
      `;

      const input = div.querySelector('input, select, textarea');
      input.addEventListener('change', (e) => {
        const key = e.target.dataset.key;
        let value = e.target.value;
        if (field.type === 'number') value = parseFloat(value) || 0;
        step.config[key] = value;
        this.saveToStorage();
      });

      container.appendChild(div);
    });

    if (step.type === 'condition') {
      const branchDiv = document.createElement('div');
      branchDiv.className = 'form-group';
      branchDiv.innerHTML = `
        <label>Kết nối rẽ nhánh</label>
        <div class="conditional-branches">
          <button class="branch-btn yes ${this.hasConnection(step.id, 'yes') ? 'active' : ''}" onclick="workflow.toggleBranch(${step.id}, 'yes')">
            ✅ Yes → ${this.getConnectionTarget(step.id, 'yes') || 'chưa chọn'}
          </button>
          <button class="branch-btn no ${this.hasConnection(step.id, 'no') ? 'active' : ''}" onclick="workflow.toggleBranch(${step.id}, 'no')">
            ❌ No → ${this.getConnectionTarget(step.id, 'no') || 'chưa chọn'}
          </button>
        </div>
      `;
      container.appendChild(branchDiv);
    }
  }

  hasConnection(fromId, label) {
    return this.connections.some(c => c.from === fromId && c.label === label);
  }

  getConnectionTarget(fromId, label) {
    const conn = this.connections.find(c => c.from === fromId && c.label === label);
    if (!conn) return null;
    const target = this.steps.find(s => s.id === conn.to);
    return target ? target.name : null;
  }

  toggleBranch(stepId, label) {
    if (this.connectionMode === 'from:' + stepId + ':' + label) {
      this.connectionMode = null;
      this.pendingConnection = null;
    } else {
      this.connectionMode = 'from:' + stepId + ':' + label;
      this.showToast('Click vào bước tiếp theo để kết nối', 'info');
    }
    this.render();
  }

  startConnection(nodeId) {
    this.connectionMode = 'from:' + nodeId;
    this.showToast('Click vào bước tiếp theo để kết nối', 'info');
  }

  handleStepClick(stepId) {
    if (!this.connectionMode) {
      this.selectStep(stepId);
      return;
    }

    const [mode, fromIdStr, label] = this.connectionMode.split(':');
    const fromId = parseInt(fromIdStr);

    if (fromId === stepId) {
      this.showToast('Không thể kết nối với chính mình', 'error');
      return;
    }

    const existingIdx = this.connections.findIndex(c => c.from === fromId && c.label === (label || 'default'));
    if (existingIdx >= 0) {
      this.connections.splice(existingIdx, 1);
    }

    this.connections.push({
      from: fromId,
      to: stepId,
      label: label || 'default',
    });

    this.connectionMode = null;
    this.pendingConnection = null;
    this.render();
    this.saveToStorage();
    this.updateInfo();
    this.showToast('Đã kết nối!', 'success');
  }

  deleteConnection(fromId, label) {
    const idx = this.connections.findIndex(c => c.from === fromId && c.label === label);
    if (idx >= 0) {
      this.connections.splice(idx, 1);
      this.render();
      this.saveToStorage();
      this.updateInfo();
    }
  }

  render() {
    const container = document.getElementById('stepsContainer');
    container.innerHTML = '';

    this.steps.forEach(step => {
      const stepType = WORKFLOW_STEP_TYPES[step.type] || WORKFLOW_STEP_TYPES.action;
      const el = document.createElement('div');
      el.className = 'step-block';
      el.dataset.id = step.id;
      el.dataset.type = step.type;

      const hasYesConn = this.hasConnection(step.id, 'yes');
      const hasNoConn = this.hasConnection(step.id, 'no');
      const hasDefaultConn = this.connections.some(c => c.from === step.id && c.label === 'default');

      el.innerHTML = `
        <div class="connector-point input" data-node-id="${step.id}"></div>
        <span class="step-block-icon">${stepType.icon}</span>
        <div class="step-block-content">
          <div class="step-block-name">${step.name}</div>
          <div class="step-block-type">${stepType.label}</div>
        </div>
        <div class="step-block-connector">
          ${step.type === 'condition' ? `
            <div class="connector-point output" data-branch="yes" data-node-id="${step.id}" style="background:${hasYesConn ? '#10b981' : ''};border-color:${hasYesConn ? '#10b981' : ''}"></div>
            <div class="connector-point output" data-branch="no" data-node-id="${step.id}" style="background:${hasNoConn ? '#ef4444' : ''};border-color:${hasNoConn ? '#ef4444' : ''}"></div>
          ` : `
            <div class="connector-point output" data-node-id="${step.id}" style="background:${hasDefaultConn ? '#3b82f6' : ''};border-color:${hasDefaultConn ? '#2563eb' : ''}"></div>
          `}
        </div>
      `;

      el.addEventListener('click', (e) => {
        if (e.target.classList.contains('connector-point')) {
          const branch = e.target.dataset.branch;
          const nodeId = parseInt(e.target.dataset.nodeId);
          if (branch) {
            this.toggleBranch(nodeId, branch);
          } else {
            this.startConnection(nodeId);
          }
        } else {
          this.handleStepClick(step.id);
        }
      });

      container.appendChild(el);
    });

    this.renderConnections();
  }

  renderConnections() {
    const svg = document.getElementById('connectionsSvg');
    const container = document.getElementById('canvasContent');
    const containerRect = container.getBoundingClientRect();

    svg.innerHTML = '';

    this.connections.forEach(conn => {
      const fromEl = document.querySelector(`[data-id="${conn.from}"] .connector-point.output${conn.label !== 'default' ? `[data-branch="${conn.label}"]` : ':not([data-branch])'}`);
      const toEl = document.querySelector(`[data-id="${conn.to}"] .connector-point.input`);
      const startEl = document.getElementById('workflowStart').querySelector('.connector-point');

      let fromRect, toRect;

      if (conn.from === 'start') {
        fromRect = startEl.getBoundingClientRect();
      } else {
        fromRect = fromEl?.getBoundingClientRect();
      }

      if (!fromRect || !toEl) return;
      toRect = toEl.getBoundingClientRect();

      const x1 = fromRect.left + fromRect.width / 2 - containerRect.left;
      const y1 = fromRect.top + fromRect.height / 2 - containerRect.top;
      const x2 = toRect.left + toRect.width / 2 - containerRect.left;
      const y2 = toRect.top + toRect.height / 2 - containerRect.top;

      const midX = (x1 + x2) / 2;
      const path = document.createElementNS('http://www.w3.org/2000/svg', 'path');
      
      let d;
      if (Math.abs(y2 - y1) < 20) {
        d = `M ${x1} ${y1} L ${x2} ${y2}`;
      } else {
        const ctrlOffset = Math.min(Math.abs(y2 - y1) * 0.5, 100);
        d = `M ${x1} ${y1} C ${x1 + ctrlOffset} ${y1}, ${x2 - ctrlOffset} ${y2}, ${x2} ${y2}`;
      }
      
      path.setAttribute('d', d);
      path.setAttribute('class', conn.label !== 'default' ? `conditional-${conn.label}` : '');
      
      const gradientId = `gradient-${conn.from}-${conn.to}`;
      const defs = document.createElementNS('http://www.w3.org/2000/svg', 'defs');
      const gradient = document.createElementNS('http://www.w3.org/2000/svg', 'linearGradient');
      gradient.id = gradientId;
      
      const color = conn.label === 'yes' ? '#10b981' : conn.label === 'no' ? '#ef4444' : '#3b82f6';
      gradient.innerHTML = `
        <stop offset="0%" stop-color="${color}" stop-opacity="0.6"/>
        <stop offset="100%" stop-color="${color}"/>
      `;
      defs.appendChild(gradient);
      svg.appendChild(defs);
      
      path.setAttribute('stroke', `url(#${gradientId})`);
      path.setAttribute('stroke-width', '2');
      path.setAttribute('fill', 'none');

      svg.appendChild(path);

      if (conn.label !== 'default') {
        const label = document.createElementNS('http://www.w3.org/2000/svg', 'text');
        label.setAttribute('x', midX);
        label.setAttribute('y', (y1 + y2) / 2 - 8);
        label.setAttribute('class', 'connection-label');
        label.setAttribute('text-anchor', 'middle');
        label.textContent = conn.label === 'yes' ? '✅ Yes' : '❌ No';
        svg.appendChild(label);
      }

      const circle = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
      circle.setAttribute('cx', x2);
      circle.setAttribute('cy', y2);
      circle.setAttribute('r', '4');
      circle.setAttribute('fill', '#3b82f6');
      svg.appendChild(circle);
    });
  }

  deleteSelected() {
    if (!this.selectedStep) return;

    this.steps = this.steps.filter(s => s.id !== this.selectedStep);
    this.connections = this.connections.filter(c => c.from !== this.selectedStep && c.to !== this.selectedStep);
    this.selectedStep = null;
    
    this.render();
    this.updatePropertiesPanel();
    this.saveToStorage();
    this.updateInfo();
    this.showToast('Đã xoá bước', 'success');
  }

  clearCanvas() {
    if (!confirm('Xoá tất cả các bước và kết nối?')) return;
    this.steps = [];
    this.connections = [];
    this.selectedStep = null;
    this.workflowId = this.generateId();
    this.render();
    this.updatePropertiesPanel();
    this.saveToStorage();
    this.updateInfo();
    this.showToast('Đã xoá canvas', 'success');
  }

  save() {
    const name = document.getElementById('workflowName').value.trim() || 'Untitled Workflow';
    const workflow = this.toJSON();
    workflow.name = name;

    this.showToast('💾 Đang lưu...', 'info');

    fetch('/api/v1/workflows', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        name: workflow.name,
        description: 'Created with Visual Workflow Builder',
        tags: ['visual-builder'],
        steps: workflow.steps.map(s => ({
          name: s.name,
          type: s.type.charAt(0).toUpperCase() + s.type.slice(1),
          config: s.config,
        })),
      }),
    })
      .then(r => r.json())
      .then(d => {
        if (d.ok || d.id) {
          this.showToast('✅ Đã lưu workflow!', 'success');
        } else {
          this.showToast('❌ Lỗi: ' + (d.error || 'Unknown'), 'error');
        }
      })
      .catch(e => {
        this.showToast('❌ Lỗi kết nối: ' + e.message, 'error');
      });
  }

  preview() {
    const modal = document.getElementById('previewModal');
    const content = document.getElementById('previewContent');
    content.textContent = JSON.stringify(this.toJSON(), null, 2);
    modal.style.display = 'flex';
  }

  closePreview() {
    document.getElementById('previewModal').style.display = 'none';
  }

  run() {
    const workflow = this.toJSON();
    if (workflow.steps.length === 0) {
      this.showToast('⚠️ Thêm ít nhất 1 bước', 'error');
      return;
    }

    this.showToast('▶️ Đang chạy workflow...', 'info');

    fetch('/api/v1/workflows/run', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ workflow, input: '' }),
    })
      .then(r => r.json())
      .then(d => {
        if (d.ok) {
          this.showToast('✅ Hoàn thành!', 'success');
          this.preview();
        } else {
          this.showToast('❌ Lỗi: ' + (d.error || 'Unknown'), 'error');
        }
      })
      .catch(e => {
        this.showToast('❌ Lỗi: ' + e.message, 'error');
      });
  }

  exportJson() {
    const data = this.toJSON();
    data.name = document.getElementById('workflowName').value.trim() || 'workflow';
    
    const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${data.name.replace(/\s+/g, '_')}.json`;
    a.click();
    URL.revokeObjectURL(url);
    
    this.showToast('📤 Đã xuất file JSON', 'success');
  }

  importJson() {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json';
    input.onchange = (e) => {
      const file = e.target.files[0];
      if (!file) return;

      const reader = new FileReader();
      reader.onload = (ev) => {
        try {
          const data = JSON.parse(ev.target.result);
          this.fromJSON(data);
          this.showToast('📥 Đã nhập workflow!', 'success');
        } catch (err) {
          this.showToast('❌ File không hợp lệ', 'error');
        }
      };
      reader.readAsText(file);
    };
    input.click();
  }

  toJSON() {
    return {
      id: this.workflowId,
      name: document.getElementById('workflowName').value.trim() || 'Workflow',
      steps: this.steps.map(s => ({
        id: s.id,
        type: s.type,
        name: s.name,
        config: s.config,
      })),
      connections: this.connections.map(c => ({
        from: c.from,
        to: c.to,
        label: c.label,
      })),
    };
  }

  fromJSON(data) {
    this.workflowId = data.id || this.generateId();
    this.steps = (data.steps || []).map(s => ({
      id: s.id || this.nextId++,
      type: s.type,
      name: s.name || 'Step',
      config: s.config || {},
    }));
    this.connections = (data.connections || []).map(c => ({
      from: c.from,
      to: c.to,
      label: c.label || 'default',
    }));
    this.nextId = Math.max(...this.steps.map(s => s.id), 0) + 1;

    document.getElementById('workflowName').value = data.name || 'Imported Workflow';
    
    this.render();
    this.updatePropertiesPanel();
    this.saveToStorage();
    this.updateInfo();
  }

  saveToStorage() {
    try {
      localStorage.setItem('workflow_builder_' + this.workflowId, JSON.stringify(this.toJSON()));
    } catch (e) {}
  }

  loadFromStorage() {
    try {
      const saved = localStorage.getItem('workflow_builder_current');
      if (saved) {
        const data = JSON.parse(saved);
        this.fromJSON(data);
      }
    } catch (e) {}
  }

  updateInfo() {
    const info = document.getElementById('workflowInfo');
    info.style.display = this.steps.length > 0 ? 'block' : 'none';
    
    document.getElementById('infoId').textContent = this.workflowId;
    document.getElementById('infoSteps').textContent = this.steps.length;
    document.getElementById('infoConnections').textContent = this.connections.length;
  }

  showToast(message, type = 'info') {
    const existing = document.querySelector('.toast');
    if (existing) existing.remove();

    const toast = document.createElement('div');
    toast.className = `toast ${type}`;
    toast.textContent = message;
    document.body.appendChild(toast);

    setTimeout(() => toast.remove(), 3000);
  }
}

let workflow;

document.addEventListener('DOMContentLoaded', () => {
  workflow = new WorkflowBuilder();

  document.getElementById('propName').addEventListener('change', (e) => {
    if (workflow.selectedStep) {
      const step = workflow.steps.find(s => s.id === workflow.selectedStep);
      if (step) {
        step.name = e.target.value;
        workflow.render();
        workflow.saveToStorage();
      }
    }
  });

  document.addEventListener('click', (e) => {
    if (!e.target.closest('.step-block') && !e.target.closest('.workflow-properties')) {
      workflow.selectedStep = null;
      workflow.updatePropertiesPanel();
      document.querySelectorAll('.step-block').forEach(el => el.classList.remove('selected'));
    }
  });

  window.addEventListener('resize', () => {
    workflow.renderConnections();
  });
});
