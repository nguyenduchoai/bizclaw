// BizClaw Welcome Onboarding — SME Friendly Setup
// Flow: Welcome → Template Select → Business Info → Auto-Training → Dashboard Ready

const { h, html, useState, useEffect, useCallback, useRef } = window;
import { authFetch } from '/static/dashboard/shared.js';

// ═══ ONBOARDING STEPS ═══
const STEPS = [
  {
    id: 'welcome',
    icon: '👋',
    title: 'Chào mừng đến BizClaw!',
    titleEn: 'Welcome to BizClaw!',
    subtitle: 'Hệ thống tự động hóa Zalo & Telesale siêu việt',
    subtitleEn: 'Ultimate Zalo & Telesale Automation',
  },
  {
    id: 'role_select',
    icon: '🎯',
    title: 'Chọn loại trợ lý',
    titleEn: 'Choose Assistant Role',
    question: 'Để bắt đầu nhanh, bạn muốn BizClaw đảm nhận công việc gì nhất trong hôm nay?',
    questionEn: 'To get started quickly, what role do you need most today?',
    options: [
      { id: 'zalo_sale', label: '🛒 Nhân viên Sale / Chốt đơn Zalo' },
      { id: 'secretary', label: '📝 Thư ký tóm tắt tin nhắn Group Zalo' },
      { id: 'customer_care', label: '🎧 Chăm sóc Khách hàng 24/7' },
      { id: 'marketing', label: '🚀 Làm Marketing & Viết Content' },
    ]
  },
  {
    id: 'business',
    icon: '🏢',
    title: 'Sản phẩm của bạn',
    titleEn: 'Your Business',
    question: 'Tuyệt vời! Bạn đang kinh doanh sản phẩm/dịch vụ gì? Tên thương hiệu là gì?',
    questionEn: 'Great! What product/service do you sell? What is your brand name?',
    placeholder: 'Ví dụ: Shop Quần áo Mèo Béo, chuyên bán đồ Pijama',
  },
  {
    id: 'generating',
    icon: '🤖',
    title: 'Đang huấn luyện AI...',
    titleEn: 'Training your AI...',
    subtitle: 'Đang thiết lập kịch bản bán hàng và nạp bộ nhớ...',
    subtitleEn: 'Setting up sales scripts and loading memory...',
  },
  {
    id: 'complete',
    icon: '🎉',
    title: 'Hoàn tất lắp ráp!',
    titleEn: 'All Set!',
    subtitle: 'Nhân viên AI của bạn đã vào vị trí sẵn sàng.',
    subtitleEn: 'Your AI employee is ready for deployment.',
  },
];

// ═══ ONBOARDING WIZARD COMPONENT ═══
export function OnboardingWizard({ lang, onComplete }) {
  const [step, setStep] = useState(0);
  const [answers, setAnswers] = useState({});
  const [input, setInput] = useState('');
  const [generating, setGenerating] = useState(false);
  const [generatedFiles, setGeneratedFiles] = useState([]);
  const [error, setError] = useState('');
  const [progress, setProgress] = useState(0);
  const [chatHistory, setChatHistory] = useState([]);
  const chatEndRef = useRef(null);
  const inputRef = useRef(null);
  const isVi = lang !== 'en';

  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [chatHistory, step]);

  useEffect(() => {
    if (STEPS[step]?.placeholder) {
      setTimeout(() => inputRef.current?.focus(), 300);
    }
  }, [step]);

  const addAiMessage = useCallback((text) => {
    setChatHistory(prev => [...prev, { role: 'ai', text, time: Date.now() }]);
  }, []);

  const addUserMessage = useCallback((text) => {
    setChatHistory(prev => [...prev, { role: 'user', text, time: Date.now() }]);
  }, []);

  const handleNext = useCallback(async (optionValue = null) => {
    const currentStep = STEPS[step];
    const val = optionValue || input.trim();

    if (currentStep.id === 'welcome') {
      setStep(1);
      const q = isVi ? STEPS[1].question : STEPS[1].questionEn;
      addAiMessage(q);
      return;
    }

    if (currentStep.question && val) {
      addUserMessage(val);
      setAnswers(prev => ({ ...prev, [currentStep.id]: val }));
      setInput('');

      const nextIdx = step + 1;
      const nextStep = STEPS[nextIdx];

      if (nextStep && nextStep.question) {
        const acks = isVi
          ? ['Tuyệt vời! 👍', 'Đã lưu lại! 📝', 'Ok bạn! ✨']
          : ['Great! 👍', 'Saved! 📝', 'Awesome! ✨'];
        const ack = acks[Math.floor(Math.random() * acks.length)];
        
        setTimeout(() => {
          addAiMessage(ack);
          setTimeout(() => {
            addAiMessage(isVi ? nextStep.question : nextStep.questionEn);
            setStep(nextIdx);
          }, 400);
        }, 300);
      } else if (nextStep && nextStep.id === 'generating') {
        setStep(nextIdx);
        setGenerating(true);
        await generateBrainFiles({ ...answers, [currentStep.id]: val });
      }
      return;
    }

    if (!currentStep.question) {
      setStep(step + 1);
    }
  }, [step, input, answers, isVi, addAiMessage, addUserMessage]);

  const handleKeyDown = useCallback((e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleNext();
    }
  }, [handleNext]);

  const generateBrainFiles = useCallback(async (allAnswers) => {
    setError('');
    setProgress(10);

    try {
      addAiMessage(isVi ? '🧠 Đang nạp kịch bản bán hàng và phân tích ngành nghề...' : '🧠 Loading scripts and analyzing industry...');
      setProgress(30);

      // We still generate the complex Markdown logic in the backend but we hide it from SME
      const roleHint = allAnswers.role_select?.includes('Zalo') 
          ? 'Zalo Sales Assistant, aggressive closing, short messages.' 
          : 'General assistant';

      const personalizeRes = await authFetch('/api/v1/brain/personalize', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          about_user: `SME Owner`,
          agent_vibe: roleHint,
          agent_name: 'BizClaw AI',
          language: isVi ? 'vi' : 'en',
        }),
      });
      
      const personalizeData = await personalizeRes.json();
      if (personalizeData.ok) {
        setProgress(50);
        addAiMessage(isVi ? '✅ Đã nạp xong bộ nhận diện thương hiệu.' : '✅ Brand identity loaded.');
      }

      // Generate the silent config files
      const extraFiles = [
        { name: 'MEMORY.md', content: generateMemoryMd(allAnswers, isVi) },
        { name: 'TOOLS.md', content: generateToolsMd(allAnswers, isVi) },
        { name: 'AGENTS.md', content: generateAgentsMd(allAnswers, isVi) },
        { name: 'SECURITY.md', content: generateSecurityMd(isVi) },
      ];

      let completed = 0;
      for (const file of extraFiles) {
        try {
          const res = await authFetch(`/api/v1/brain/files/${encodeURIComponent(file.name)}`, {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ content: file.content }),
          });
          if ((await res.json()).ok) completed++;
        } catch (e) {
          console.warn(`Failed to save ${file.name}`);
        }
        setProgress(50 + Math.round((completed / extraFiles.length) * 50));
      }

      setProgress(100);
      addAiMessage(isVi
        ? `🎉 Xong! Trợ lý đã sẵn sàng. Bạn chỉ cần quét mã QR Zalo trên Dashboard nữa là chạy.`
        : `🎉 Done! Assistant is ready. You just need to scan Zalo QR on Dashboard to run.`
      );

      localStorage.setItem('bizclaw_onboarded', 'true');
      localStorage.setItem('bizclaw_onboard_time', new Date().toISOString());

      setTimeout(() => {
        setGenerating(false);
        setStep(STEPS.length - 1);
      }, 2000);

    } catch (e) {
      setError(e.message || 'Generation failed');
      setGenerating(false);
      addAiMessage(isVi ? '❌ Có lỗi xảy ra, có thể mạng hơi chậm. Đang đưa bạn vào vùng làm việc ngay.' : '❌ Network error. Going to dashboard.');
      setTimeout(() => setStep(STEPS.length - 1), 2000);
    }
  }, [isVi, addAiMessage]);

  const current = STEPS[step];
  const progressPct = current.id === 'generating' ? progress : Math.round((step / (STEPS.length - 1)) * 100);

  return html`
    <div class="onboarding-overlay">
      <div class="onboarding-container">
        
        <div class="onboarding-progress">
          <div class="onboarding-progress-bar" style=${{ width: progressPct + '%' }}></div>
        </div>

        ${current.id === 'welcome' && html`
          <div class="onboarding-welcome" key="welcome">
            <div class="onboarding-logo" style="margin-bottom:20px; text-align:center;">
              <span style="font-size: 3rem;">🦁</span>
              <h2 style="margin: 0; color: #fff; font-size: 1.5rem;">BizClaw</h2>
            </div>
            <h1 class="onboarding-title">${isVi ? current.title : current.titleEn}</h1>
            <p class="onboarding-subtitle">${isVi ? current.subtitle : current.subtitleEn}</p>
            
            <div class="onboarding-features">
              <div class="onboarding-feature">
                <span class="feature-icon">💬</span>
                <div>
                  <strong>${isVi ? 'Trực Zalo 24/7' : '24/7 Zalo Team'}</strong>
                  <p>${isVi ? 'Tự động chốt đơn trên nhiều Zalo cá nhân cùng lúc' : 'Auto close deals on Zalo immediately'}</p>
                </div>
              </div>
              <div class="onboarding-feature">
                <span class="feature-icon">🚀</span>
                <div>
                  <strong>${isVi ? '0 Phút Đào Tạo' : '0 Mins Training'}</strong>
                  <p>${isVi ? 'Chỉ cần chọn Template, AI tự sinh ra 100 kịch bản bán hàng' : 'Choose a template, AI generates 100 sales scripts'}</p>
                </div>
              </div>
            </div>

            <button class="onboarding-btn-primary" onClick=${() => handleNext()}>
              ${isVi ? 'Bắt Đầu Setup Nhanh 🚀' : 'Start Quick Setup 🚀'}
            </button>
            <button class="onboarding-btn-skip" onClick=${() => {
              localStorage.setItem('bizclaw_onboarded', 'true');
              onComplete?.();
            }}>
              ${isVi ? 'Bỏ qua, tôi là dân IT →' : 'Skip, I am an IT Pro →'}
            </button>
          </div>
        `}

        ${(current.question || current.id === 'generating') && html`
          <div class="onboarding-chat" key="chat">
            <div class="onboarding-chat-header">
              <span class="onboarding-chat-avatar">🤖</span>
              <div>
                <strong>BizClaw Setup Master</strong>
                <span class="onboarding-chat-status">● ${isVi ? 'Đang online hỗ trợ bạn' : 'Online assisting you'}</span>
              </div>
            </div>

            <div class="onboarding-chat-messages">
              ${chatHistory.map((msg, i) => html`
                <div key=${i} class=${`onboarding-msg onboarding-msg-${msg.role}`}>
                  ${msg.role === 'ai' && html`<span class="onboarding-msg-avatar">🤖</span>`}
                  <div class="onboarding-msg-bubble">${msg.text}</div>
                </div>
              `)}
              
              ${!generating && current.options && html`
                 <div class="onboarding-options" style="display: flex; flex-direction: column; gap: 8px; margin-left: 40px; margin-top: 10px;">
                    ${current.options.map(opt => html`
                       <button 
                         class="onboarding-btn-option" 
                         style="text-align: left; background: #2d3748; color: #fff; border: 1px solid #4a5568; outline: none; padding: 12px; border-radius: 8px; cursor: pointer; transition: all 0.2s;"
                         onMouseOver=${e => Object.assign(e.target.style, {background: '#3182ce', borderColor: '#63b3ed'})}
                         onMouseOut=${e => Object.assign(e.target.style, {background: '#2d3748', borderColor: '#4a5568'})}
                         onClick=${() => handleNext(opt.label)}
                       >
                         ${opt.label}
                       </button>
                    `)}
                 </div>
              `}

              ${generating && html`
                <div class="onboarding-msg onboarding-msg-ai">
                  <span class="onboarding-msg-avatar">🤖</span>
                  <div class="onboarding-msg-bubble onboarding-typing">
                    <span></span><span></span><span></span>
                  </div>
                </div>
              `}
              <div ref=${chatEndRef}></div>
            </div>

            ${!generating && current.placeholder && html`
              <div class="onboarding-chat-input">
                <textarea
                  ref=${inputRef}
                  value=${input}
                  onInput=${e => setInput(e.target.value)}
                  onKeyDown=${handleKeyDown}
                  placeholder=${current.placeholder || ''}
                  rows="2"
                  class="onboarding-textarea"
                ></textarea>
                <button
                  class="onboarding-send-btn"
                  onClick=${() => handleNext()}
                  disabled=${!input.trim()}
                >
                  ➤
                </button>
              </div>
            `}
          </div>
        `}

        ${current.id === 'complete' && html`
          <div class="onboarding-complete" key="complete">
            <div class="onboarding-complete-icon" style="font-size: 5rem;">🦁</div>
            <h1 class="onboarding-title">${isVi ? current.title : current.titleEn}</h1>
            <p class="onboarding-subtitle">${isVi ? current.subtitle : current.subtitleEn}</p>

            <div class="onboarding-summary" style="margin-top: 2rem; background: #2d3748; padding: 20px; border-radius: 12px;">
              <p>🎯 <strong>${isVi ? 'Việc tiếp theo bạn cần làm:' : 'Next steps:'}</strong></p>
              <ul style="text-align: left; line-height: 1.8; color: #cbd5e0;">
                <li>1. Quét mã QR Zalo trong mục "Channels" để hệ thống truy cập tin nhắn.</li>
                <li>2. Bật "Auto-Reply" trong bảng điều khiển trung tâm.</li>
                <li>3. Đi pha một ly cà phê và nhìn AI chốt đơn! ☕</li>
              </ul>
            </div>

            <button class="onboarding-btn-primary" onClick=${() => onComplete?.()} style="margin-top: 2rem;">
              ${isVi ? 'Đóng và Quản Lý Ngay 🚀' : 'Go to Dashboard 🚀'}
            </button>
          </div>
        `}
      </div>
    </div>
  `;
}

// ═══ SME FOCUSED GENERATOR HACKS ═══

function generateMemoryMd(answers, isVi) {
  return [
    '# MEMORY.md',
    isVi ? '## Khách hàng doanh nghiệp' : '## Business Client',
    `- Dịch vụ/Sản phẩm: ${answers.business || 'Retail'}`,
    `- Vai trò: ${answers.role_select || 'Zalo Sales Automation'}`,
  ].join('\n');
}

function generateToolsMd(answers, isVi) {
  // Always inject Zalo natively for SME
  return [
    '# TOOLS.md',
    '- `zalo_tool`',
    '- `social_post`',
    '- `web_search`'
  ].join('\n');
}

function generateAgentsMd(answers, isVi) {
  return [
    '# AGENTS.md',
    '- **Zalo (Native)**: Ưu tiên trả lời nhanh.',
  ].join('\n');
}

function generateSecurityMd(isVi) {
  return [
    '# SECURITY.md',
    '- Không giảm giá tùy tiện, không chia sẻ mật khẩu.',
  ].join('\n');
}
