// ChatPage — Main chat interface
const { h, html, useState, useEffect, useContext, useCallback, useRef, useMemo } = window;
import { t, authFetch, authHeaders, StatsCard } from '/static/dashboard/shared.js';

function ChatPage({ config, lang }) {
  const [messages, setMessages] = useState(() => {
    try {
      const saved = localStorage.getItem('bizclaw_chat_messages');
      if (saved) return JSON.parse(saved);
    } catch (e) {}
    return [];
  });
  const [input, setInput] = useState('');
  const [thinking, setThinking] = useState(false);
  const [streamContent, setStreamContent] = useState('');
  const [streamReqId, setStreamReqId] = useState(null);
  const [sessions, setSessions] = useState(() => {
    try {
      const saved = localStorage.getItem('bizclaw_chat_sessions');
      if (saved) return JSON.parse(saved);
    } catch (e) {}
    return [{ id: 'main', name: 'Main Chat', icon: '🤖', time: 'now', count: 0, mode: '1v1' }];
  });

  useEffect(() => {
    localStorage.setItem('bizclaw_chat_messages', JSON.stringify(messages));
  }, [messages]);

  useEffect(() => {
    localStorage.setItem('bizclaw_chat_sessions', JSON.stringify(sessions));
  }, [sessions]);
  const [activeSession, setActiveSession] = useState('main');
  const activeSessionObj = sessions.find(s => s.id === activeSession);
  const isGroupMode = activeSessionObj?.mode === 'group';
  const [wsInfo, setWsInfo] = useState({});
  const messagesEndRef = useRef(null);
  const inputRef = useRef(null);
  // Multi-agent support
  const [agentsList, setAgentsList] = useState([]);
  const [selectedAgent, setSelectedAgent] = useState(''); // '' = default agent

  // Auto-create session when switching to a different agent
  const handleAgentSwitch = useCallback((newAgent) => {
    setSelectedAgent(newAgent);
    if (newAgent && newAgent !== '__broadcast__') {
      // Check if session for this agent already exists
      const agentSessionId = `agent_${newAgent}`;
      const existingSession = sessions.find(s => s.id === agentSessionId);
      if (!existingSession) {
        // Create new session for this agent
        const agentInfo = agentsList.find(a => a.name === newAgent);
        const newSession = {
          id: agentSessionId,
          name: newAgent,
          icon: agentInfo?.role === 'coder' ? '💻' : agentInfo?.role === 'writer' ? '✍️' : agentInfo?.role === 'analyst' ? '📊' : '🤖',
          time: new Date().toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit' }),
          count: 0,
          mode: '1v1',
          agentName: newAgent
        };
        setSessions(prev => [...prev, newSession]);
        setActiveSession(agentSessionId);
        setMessages([]); // Clear chat for new agent session
      } else {
        // Switch to existing session
        setActiveSession(agentSessionId);
        // Load messages for this session (stored separately)
        try {
          const savedMsgs = localStorage.getItem(`bizclaw_chat_${agentSessionId}`);
          if (savedMsgs) setMessages(JSON.parse(savedMsgs));
          else setMessages([]);
        } catch(e) { setMessages([]); }
      }
    }
  }, [sessions, agentsList]);

  // Save messages per session
  useEffect(() => {
    if (activeSession && activeSession !== 'main') {
      localStorage.setItem(`bizclaw_chat_${activeSession}`, JSON.stringify(messages));
    }
  }, [messages, activeSession]);

  // Fetch available agents
  useEffect(() => {
    (async () => {
      try {
        const r = await authFetch('/api/v1/agents');
        const d = await r.json();
        setAgentsList(d.agents || []);
      } catch(e) { console.warn('Agents fetch:', e); }
    })();
  }, []);

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
          setMessages(prev => [...prev, { type: 'bot', content: fullContent, provider: msg.provider, model: msg.model, mode: msg.mode, context: msg.context, agent: msg.agent }]);
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
    // @mention support — detect @agentname at start of message in group chat
    let targetAgent = selectedAgent;
    let displayText = text;
    const mentionMatch = text.match(/^@(\S+)\s+(.*)/);
    if (mentionMatch && isGroupMode) {
      const mentionedName = mentionMatch[1];
      const found = agentsList.find(a => a.name.toLowerCase() === mentionedName.toLowerCase());
      if (found) {
        targetAgent = found.name;
        displayText = mentionMatch[2];
      }
    }
    // In group mode, auto-broadcast if no specific agent selected and no @mention
    if (isGroupMode && !targetAgent) targetAgent = '__broadcast__';
    setMessages(prev => [...prev, { type: 'user', content: text, agent: targetAgent || undefined }]);
    setThinking(true);

    // Send via WebSocket — include agent name for multi-agent routing
    if (window._ws && window._ws.readyState === 1) {
      const payload = { type: 'chat', content: text, stream: true };
      if (targetAgent && targetAgent !== '__broadcast__') payload.agent = targetAgent;
      
      if (targetAgent === '__broadcast__') {
        // Broadcast mode: send to ALL registered agents
        if (agentsList.length === 0) {
          setMessages(prev => [...prev, { type: 'system', content: '⚠️ No agents registered. Create agents first in AI Agent page.', error: true }]);
          setThinking(false);
          return;
        }
        agentsList.forEach(a => {
          window._ws.send(JSON.stringify({ type: 'chat', content: displayText || text, stream: true, agent: a.name }));
        });
      } else {
        window._ws.send(JSON.stringify(payload));
      }
    } else {
      setMessages(prev => [...prev, { type: 'system', content: '🔴 WebSocket not connected. Reconnecting...', error: true }]);
      setThinking(false);
    }
  };

  // Render markdown content (code blocks, inline code, bold, italic, headers, lists, links)
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
        return html`<div key=${i} style="background:var(--bg);border:1px solid var(--border);border-radius:8px;margin:8px 0;overflow-x:auto">
          ${lang && html`<div style="padding:5px 12px;font-size:10px;color:var(--text2);border-bottom:1px solid var(--border);text-transform:uppercase;font-weight:600;letter-spacing:.5px;display:flex;align-items:center;gap:6px">
            <span style="color:var(--accent2)">●</span> ${lang}
          </div>`}
          <pre style="padding:12px 16px;font-size:12px;font-family:var(--mono);white-space:pre-wrap;word-break:break-all;margin:0;color:var(--cyan);line-height:1.6">${code}</pre>
        </div>`;
      }
      // Process inline markdown
      let processed = part
        .replace(/#### (.+)/g, '<h4 style="font-size:13px;font-weight:700;margin:8px 0 4px;color:var(--accent2)">$1</h4>')
        .replace(/### (.+)/g, '<h3 style="font-size:14px;font-weight:700;margin:8px 0 4px;color:var(--text)">$1</h3>')
        .replace(/## (.+)/g, '<h2 style="font-size:15px;font-weight:700;margin:10px 0 6px;color:var(--text)">$1</h2>')
        .replace(/# (.+)/g, '<h1 style="font-size:17px;font-weight:700;margin:10px 0 6px;color:var(--text)">$1</h1>')
        .replace(/`([^`]+)`/g, '<code style="background:rgba(99,102,241,.1);color:var(--accent2);padding:1px 5px;border-radius:4px;font-family:var(--mono);font-size:12px">$1</code>')
        .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
        .replace(/\*(.*?)\*/g, '<em>$1</em>')
        .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" target="_blank" style="color:var(--accent2);text-decoration:underline">$1</a>');
      // Convert bullet lists
      processed = processed.replace(/^- (.+)$/gm, '<li style="margin:2px 0;list-style:disc;margin-left:16px">$1</li>');
      processed = processed.replace(/^\d+\. (.+)$/gm, '<li style="margin:2px 0;list-style:decimal;margin-left:16px">$1</li>');
      processed = processed.replace(/\n/g, '<br/>');
      return html`<span key=${i} dangerouslySetInnerHTML=${{ __html: processed }} />`;
    });
  };

  const fmtTime = () => new Date().toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit' });

  return html`<div style="height:calc(100vh - 56px);display:flex;flex-direction:column">
    <div class="chat-layout" style="flex:1;height:100%">
      <!-- Sidebar: conversation list -->
      <div class="chat-sidebar">
        <div class="chat-sidebar-header">
          <h3>💬 ${t('chat.title', lang)}</h3>
          <div style="display:flex;gap:4px">
            <button class="btn btn-outline btn-sm" onClick=${() => {
              const id = 'chat_' + Date.now();
              setSessions(prev => [{ id, name: 'New Chat', icon: '💬', time: fmtTime(), count: 0, mode: '1v1' }, ...prev]);
              setActiveSession(id);
              setSelectedAgent('');
              setMessages([]);
            }} title="New 1-on-1 Chat">+</button>
            <button class="btn btn-sm" style="background:var(--accent2);color:#fff;font-size:11px" onClick=${() => {
              const id = 'group_' + Date.now();
              setSessions(prev => [{ id, name: 'Group Chat', icon: '👥', time: fmtTime(), count: 0, mode: 'group' }, ...prev]);
              setActiveSession(id);
              setSelectedAgent('__broadcast__');
              setMessages([{type:'system',content:'👥 Group Chat — All agents will respond. Use @agentname to target a specific agent.'}]);
            }} title="New Group Chat">👥</button>
          </div>
        </div>
        <div class="chat-list">
          <div class="chat-list-sep">Sessions</div>
          ${sessions.map(s => html`
            <div key=${s.id} class="chat-list-item ${activeSession === s.id ? 'active' : ''}" onClick=${() => { 
              setActiveSession(s.id); 
              if(s.mode==='group') { setSelectedAgent('__broadcast__'); setMessages([]); }
              else if (s.agentName) { setSelectedAgent(s.agentName); try { const m = localStorage.getItem(`bizclaw_chat_${s.id}`); setMessages(m ? JSON.parse(m) : []); } catch(e) { setMessages([]); } }
              else { setSelectedAgent(''); try { const m = localStorage.getItem(`bizclaw_chat_${s.id}`); setMessages(m ? JSON.parse(m) : []); } catch(e) { setMessages([]); } }
            }}>
              <div class="chat-list-icon">${s.icon}</div>
              <div class="chat-list-info">
                <div class="chat-list-name">${s.name} ${s.mode==='group' ? html`<span class="badge badge-blue" style="font-size:9px;padding:1px 5px;margin-left:4px">GROUP</span>`:''}</div>
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
          <div class="chat-target" style="display:flex;align-items:center;gap:10px">
            <span class="chat-target-icon">🤖</span>
            <div>
              <div class="chat-target-name">${selectedAgent ? (agentsList.find(a=>a.name===selectedAgent)?.name || selectedAgent) : (config?.agent_name || 'BizClaw AI')}</div>
              <div class="chat-target-sub">${wsInfo.provider || config?.default_provider || '—'} · ${wsInfo.model || '—'}${wsInfo.agent_engine ? ' · 🧠 Agent' : ''}</div>
            </div>
            ${agentsList.length > 0 ? html`
              <select value=${selectedAgent} onChange=${e=>handleAgentSwitch(e.target.value)}
                style="padding:4px 8px;font-size:12px;border-radius:6px;border:1px solid var(--border);background:var(--bg2);color:var(--text);cursor:pointer;min-width:140px">
                <option value="">🤖 Default Agent</option>
                ${agentsList.map(a => html`<option key=${a.name} value=${a.name}>${a.role === 'coder' ? '💻' : a.role === 'writer' ? '✍️' : a.role === 'analyst' ? '📊' : '🤖'} ${a.name}</option>`)}
                <option value="__broadcast__">📢 All Agents (Broadcast)</option>
              </select>
            ` : ''}
          </div>
          <div style="display:flex;gap:6px;align-items:center">
            <span class="badge ${thinking ? 'badge-yellow pulse' : 'badge-green'}">${thinking ? '⏳ thinking' : '● ready'}</span>
            ${activeSession !== 'main' ? html`<button class="btn btn-outline btn-sm" onClick=${() => { 
              const newId = `chat_${Date.now()}`;
              const newSession = { id: newId, name: 'Chat ' + sessions.length, icon: '💬', time: 'now', count: 0, mode: '1v1' };
              setSessions(prev => [...prev, newSession]);
              setActiveSession(newId);
              setMessages([]);
              setSelectedAgent('');
            }} title="New Chat">➕</button>` : ''}
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
                ${m.type === 'bot' && m.agent && isGroupMode ? html`<div style="font-size:11px;font-weight:600;color:var(--accent2);margin-bottom:4px;display:flex;align-items:center;gap:4px">
                  <span style="display:inline-block;width:18px;height:18px;border-radius:50%;background:var(--accent);color:#fff;text-align:center;font-size:10px;line-height:18px">${m.agent.charAt(0).toUpperCase()}</span>
                  ${m.agent}
                </div>` : ''}
                ${m.type === 'bot' ? renderContent(m.content) : m.content}
                ${m.type === 'bot' ? html`<div style="font-size:10px;color:var(--text2);margin-top:4px;text-align:right">
                  ${m.agent && !isGroupMode ? '🤖 ' + m.agent : ''}${m.mode === 'agent' ? ' 🧠 Agent' : ''}${m.mode === 'multi-agent' ? ' 🔀 Multi-Agent' : ''}${m.context ? ' · ctx:' + m.context.total_tokens : ''}
                </div>` : ''}
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
            placeholder=${isGroupMode ? 'Type a message or @agentname to target...' : t('chat.placeholder', lang)} autocomplete="off" />
          <button onClick=${sendMessage} disabled=${thinking}>${isGroupMode ? '📢 Send' : t('chat.send', lang)}</button>
        </div>
      </div>
    </div>
  </div>`;
}


export { ChatPage };
