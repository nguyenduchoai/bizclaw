// SME Dashboard Page — Simplified view for non-technical users
const { h, html, useState, useEffect, useContext, useCallback, useRef, useMemo } = window;
import { t, authFetch, StatsCard } from '/static/dashboard/shared.js';

function SmeCard({ icon, value, label, action, actionLabel, color = 'accent' }) {
    return html`
        <div class="sme-card">
            <div class="sme-card-icon">${icon}</div>
            <div class="sme-card-value" style="color: var(--${color})">${value}</div>
            <div class="sme-card-label">${label}</div>
            <div class="sme-card-action" onClick=${action}>
                ${actionLabel} →
            </div>
        </div>
    `;
}

function SmeQuickActions({ lang, navigate }) {
    const actions = [
        { icon: '🤖', label: 'New Agent', page: 'agents' },
        { icon: '🔄', label: 'New Workflow', page: 'workflows' },
        { icon: '📥', label: 'Import Knowledge', page: 'knowledge' },
        { icon: '📤', label: 'Export', page: 'analytics' },
    ];

    return html`
        <div class="sme-quick-actions">
            <div class="sme-quick-actions-title">
                📈 Quick Actions
            </div>
            <div class="sme-quick-actions-grid">
                ${actions.map(a => html`
                    <button class="sme-action-btn" onClick=${() => navigate(a.page)}>
                        <span class="icon">${a.icon}</span>
                        ${a.label}
                    </button>
                `)}
            </div>
        </div>
    `;
}

function DevModeBanner({ onEnable }) {
    return html`
        <div class="dev-mode-banner">
            <div class="dev-mode-banner-text">
                <span class="icon">🔧</span>
                <span>Developer Mode — Full technical dashboard available</span>
            </div>
            <button class="dev-mode-btn" onClick=${onEnable}>
                Switch to Developer Mode
            </button>
        </div>
    `;
}

function SmeDashboardPage({ config, lang }) {
    const { navigate } = useContext(AppContext);
    const [stats, setStats] = useState({
        agents: 0,
        tasksToday: 0,
        unreadMessages: 0,
    });
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        (async () => {
            try {
                const [agentsRes, tracesRes] = await Promise.allSettled([
                    authFetch('/api/v1/agents'),
                    authFetch('/api/v1/traces'),
                ]);

                let agentsCount = 0;
                let tasksToday = 0;
                let unreadMessages = 0;

                if (agentsRes.status === 'fulfilled') {
                    const d = await agentsRes.value.json();
                    agentsCount = (d.agents || []).length;
                }

                if (tracesRes.status === 'fulfilled') {
                    const d = await tracesRes.value.json();
                    const traces = d.traces || [];
                    const today = new Date().setHours(0, 0, 0, 0);
                    tasksToday = traces.filter(t => new Date(t.timestamp || 0).getTime() >= today).length;
                }

                setStats({
                    agents: agentsCount,
                    tasksToday,
                    unreadMessages,
                });
            } catch (e) {
                console.warn('SME Dashboard fetch:', e);
            }
            setLoading(false);
        })();
    }, []);

    const handleEnableDevMode = useCallback(() => {
        localStorage.setItem('bizclaw_sme_mode', 'false');
        window.location.reload();
    }, []);

    if (loading) {
        return html`
            <div style="display:flex;align-items:center;justify-content:center;padding:60px;color:var(--text2)">
                <div style="text-align:center">
                    <div style="font-size:32px;margin-bottom:12px;animation:pulse 1s infinite">⏳</div>
                    <div>Loading...</div>
                </div>
            </div>
        `;
    }

    return html`
        <div>
            <div class="page-header">
                <div>
                    <h1>👋 Welcome to BizClaw SME</h1>
                    <div class="sub">Simplified dashboard for managing your AI agents</div>
                </div>
            </div>

            <${DevModeBanner} onEnable=${handleEnableDevMode} />

            <div class="sme-dashboard-grid">
                <${SmeCard}
                    icon="🤖"
                    value="${stats.agents} active"
                    label="My Agents"
                    action=${() => navigate('agents')}
                    actionLabel="Manage"
                    color="accent"
                />
                <${SmeCard}
                    icon="📊"
                    value="${stats.tasksToday} tasks done"
                    label="Today's Work"
                    action=${() => navigate('activity')}
                    actionLabel="View"
                    color="green"
                />
                <${SmeCard}
                    icon="💬"
                    value="${stats.unreadMessages} unread"
                    label="Messages"
                    action=${() => navigate('chat')}
                    actionLabel="Open"
                    color="blue"
                />
            </div>

            <${SmeQuickActions} lang=${lang} navigate=${navigate} />
        </div>
    `;
}

export { SmeDashboardPage };
