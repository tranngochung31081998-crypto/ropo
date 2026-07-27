import React, { useState, useRef, useEffect } from 'react';
import { Send, Copy, Trash, Sparkles, RefreshCw, ChevronDown, Check, MessageSquare, Bot, User, Cpu } from 'lucide-react';
import { sendRouterChat, getRouterModels, type RouterModel, type ChatMessage } from '../../api/routerClient';
import { useRouterStore } from './store';
import { ProviderLogo } from './providerLogos';

const QVERIS_FALLBACK_MODELS: RouterModel[] = [
  { id: 'deepseek-r1', object: 'model', created: 1730000000, owned_by: 'deepseek', display_name: 'DeepSeek R1', description: '', provider: 'qveris' },
  { id: 'deepseek-v3.2', object: 'model', created: 1730000001, owned_by: 'deepseek', display_name: 'DeepSeek V3.2', description: '', provider: 'qveris' },
  { id: 'gpt-5.2', object: 'model', created: 1730000002, owned_by: 'openai', display_name: 'GPT-5.2', description: '', provider: 'qveris' },
  { id: 'gpt-4.1', object: 'model', created: 1730000003, owned_by: 'openai', display_name: 'GPT-4.1', description: '', provider: 'qveris' },
  { id: 'claude-3-7', object: 'model', created: 1730000004, owned_by: 'anthropic', display_name: 'Claude 3.7 Sonnet', description: '', provider: 'qveris' },
  { id: 'claude-opus-4-5', object: 'model', created: 1730000005, owned_by: 'anthropic', display_name: 'Claude Opus 4.5', description: '', provider: 'qveris' },
  { id: 'gemini-2.5-flash-image', object: 'model', created: 1730000006, owned_by: 'google', display_name: 'Gemini 2.5 Flash Image', description: '', provider: 'qveris' },
  { id: 'gemini-3-pro-image-preview', object: 'model', created: 1730000007, owned_by: 'google', display_name: 'Gemini 3 Pro Image Preview', description: '', provider: 'qveris' },
  { id: 'openai/gpt-5.6-luna', object: 'model', created: 1730000008, owned_by: 'openrouter', display_name: 'GPT-5.6 Luna', description: '', provider: 'qveris' },
  { id: 'openai/gpt-5.6-terra', object: 'model', created: 1730000009, owned_by: 'openrouter', display_name: 'GPT-5.6 Terra', description: '', provider: 'qveris' },
  { id: 'openai/gpt-5.6-sol', object: 'model', created: 1730000010, owned_by: 'openrouter', display_name: 'GPT-5.6 Sol', description: '', provider: 'qveris' },
  { id: 'anthropic/claude-opus-4.8', object: 'model', created: 1730000011, owned_by: 'openrouter', display_name: 'Claude Opus 4.8', description: '', provider: 'qveris' },
  { id: 'anthropic/claude-sonnet-5', object: 'model', created: 1730000012, owned_by: 'openrouter', display_name: 'Claude Sonnet 5', description: '', provider: 'qveris' },
  { id: 'anthropic/claude-fable-5', object: 'model', created: 1730000013, owned_by: 'openrouter', display_name: 'Claude Fable 5', description: '', provider: 'qveris' },
  { id: 'deepseek/deepseek-v4-pro', object: 'model', created: 1730000014, owned_by: 'openrouter', display_name: 'DeepSeek V4 Pro', description: '', provider: 'qveris' },
  { id: 'deepseek/deepseek-v4-flash', object: 'model', created: 1730000015, owned_by: 'openrouter', display_name: 'DeepSeek V4 Flash', description: '', provider: 'qveris' },
  { id: 'google/gemini-3.1-pro-preview', object: 'model', created: 1730000016, owned_by: 'openrouter', display_name: 'Gemini 3.1 Pro Preview', description: '', provider: 'qveris' },
  { id: 'google/gemini-3.1-flash-lite', object: 'model', created: 1730000017, owned_by: 'openrouter', display_name: 'Gemini 3.1 Flash Lite', description: '', provider: 'qveris' },
  { id: 'x-ai/grok-4.5', object: 'model', created: 1730000018, owned_by: 'openrouter', display_name: 'Grok 4.5', description: '', provider: 'qveris' },
  { id: 'qwen/qwen3.7-plus', object: 'model', created: 1730000019, owned_by: 'openrouter', display_name: 'Qwen 3.7 Plus', description: '', provider: 'qveris' },
  { id: 'moonshotai/kimi-k3', object: 'model', created: 1730000020, owned_by: 'openrouter', display_name: 'Kimi K3', description: '', provider: 'qveris' },
  { id: 'moonshotai/kimi-k2.6', object: 'model', created: 1730000021, owned_by: 'openrouter', display_name: 'Kimi K2.6', description: '', provider: 'qveris' },
  { id: 'moonshotai/kimi-k2.7-code', object: 'model', created: 1730000022, owned_by: 'openrouter', display_name: 'Kimi K2.7 Code', description: '', provider: 'qveris' },
  { id: 'z-ai/glm-5.2', object: 'model', created: 1730000023, owned_by: 'openrouter', display_name: 'GLM 5.2', description: '', provider: 'qveris' },
  { id: 'minimax/minimax-m3', object: 'model', created: 1730000024, owned_by: 'openrouter', display_name: 'MiniMax M3', description: '', provider: 'qveris' },
  { id: 'xiaomi/mimo-v2.5-pro', object: 'model', created: 1730000025, owned_by: 'openrouter', display_name: 'Mimo V2.5 Pro', description: '', provider: 'qveris' },
];

export default function PlaygroundPanel() {
  const { customProviders, bumpRequest } = useRouterStore();
  const [models, setModels] = useState<RouterModel[]>([]);
  const [selectedModel, setSelectedModel] = useState<string>('');
  const [dropdownOpen, setDropdownOpen] = useState(false);
  const [modelSearch, setModelSearch] = useState('');

  const [systemPrompt, setSystemPrompt] = useState('You are a helpful, concise software engineering assistant. Respond with clarity and actionable code when appropriate.');
  const [userMessage, setUserMessage] = useState('');
  const [temperature, setTemperature] = useState(0.7);
  const [maxTokens, setMaxTokens] = useState(2048);

  const [assistantOutput, setAssistantOutput] = useState('');
  const [displayedOutput, setDisplayedOutput] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [meta, setMeta] = useState<{ provider: string | null; model: string | null } | null>(null);
  const [copied, setCopied] = useState(false);

  const outputRef = useRef<HTMLDivElement | null>(null);
  const typingTimerRef = useRef<number | null>(null);
  const typingIdxRef = useRef(0);

  useEffect(() => {
    let cancelled = false;
    async function loadModels() {
      try {
        const res = await getRouterModels();
        if (cancelled) return;
        const data = res.data || [];
        const customs: RouterModel[] = customProviders.flatMap(p =>
          p.models.map(m => {
            const entry: RouterModel = {
              id: m, object: 'model', created: Date.parse(p.createdAt) / 1000,
              owned_by: p.providerKey, display_name: m.includes('/') ? m.split('/').pop() || m : m,
              description: `Custom via ${p.displayName}`, provider: 'qveris',
            };
            ((entry as unknown) as { provider: string }).provider = 'custom';
            return entry;
          })
        );
        const merged = [...data, ...customs];
        setModels(merged);
        if (!selectedModel && merged.length) setSelectedModel(merged[0].id);
      } catch {
        if (cancelled) return;
        setModels(QVERIS_FALLBACK_MODELS);
        if (!selectedModel) setSelectedModel(QVERIS_FALLBACK_MODELS[0].id);
      }
    }
    loadModels();
    return () => { cancelled = true; };
  }, [customProviders]);

  useEffect(() => {
    if (outputRef.current) {
      outputRef.current.scrollTop = outputRef.current.scrollHeight;
    }
  }, [displayedOutput]);

  function startTypingEffect(fullText: string) {
    if (typingTimerRef.current) {
      window.clearInterval(typingTimerRef.current);
      typingTimerRef.current = null;
    }
    typingIdxRef.current = 0;
    setDisplayedOutput('');
    if (!fullText) return;
    const speed = Math.max(6, Math.min(24, Math.floor(3800 / Math.max(fullText.length, 1))));
    typingTimerRef.current = window.setInterval(() => {
      typingIdxRef.current += 1;
      const idx = typingIdxRef.current;
      if (idx >= fullText.length) {
        setDisplayedOutput(fullText);
        if (typingTimerRef.current) {
          window.clearInterval(typingTimerRef.current);
          typingTimerRef.current = null;
        }
      } else {
        setDisplayedOutput(fullText.slice(0, idx));
      }
    }, speed);
  }

  useEffect(() => {
    return () => {
      if (typingTimerRef.current) window.clearInterval(typingTimerRef.current);
    };
  }, []);

  async function handleSend() {
    if (loading) return;
    if (!selectedModel) { setError('Select a model first.'); return; }
    if (!userMessage.trim()) { setError('User message cannot be empty.'); return; }
    setError(null);
    setLoading(true);
    setAssistantOutput('');
    setDisplayedOutput('');
    setMeta(null);
    const messages: ChatMessage[] = [];
    if (systemPrompt.trim()) messages.push({ role: 'system', content: systemPrompt.trim() });
    messages.push({ role: 'user', content: userMessage.trim() });

    try {
      const out = await sendRouterChat(
        { model: selectedModel, messages, max_tokens: maxTokens, temperature, stream: true },
        (tok) => {
          setAssistantOutput(prev => {
            const next = prev + tok;
            setDisplayedOutput(next);
            return next;
          });
        },
        (m) => setMeta(m),
      );
      setAssistantOutput(out);
      setDisplayedOutput(out);
      const tokens = Math.ceil((systemPrompt.length + userMessage.length + out.length) / 4);
      bumpRequest(tokens, false);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Request failed');
    } finally {
      setLoading(false);
    }
  }

  function handleCopy() {
    if (!assistantOutput) return;
    navigator.clipboard?.writeText(assistantOutput).then(() => {
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1600);
    }).catch(() => {});
  }

  function handleClear() {
    setAssistantOutput('');
    setDisplayedOutput('');
    setMeta(null);
    setError(null);
    setUserMessage('');
    if (typingTimerRef.current) {
      window.clearInterval(typingTimerRef.current);
      typingTimerRef.current = null;
    }
  }

  const filteredModelList = models.filter(m => {
    const s = modelSearch.trim().toLowerCase();
    if (!s) return true;
    return m.id.toLowerCase().includes(s) || m.display_name.toLowerCase().includes(s) || m.owned_by.toLowerCase().includes(s);
  });

  const selectedModelObj = models.find(m => m.id === selectedModel);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-md)', height: '100%', minHeight: 0 }}>
      <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', gap: 'var(--space-md)', flexWrap: 'wrap' }}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-2xs)' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-xs)' }}>
            <MessageSquare size={18} style={{ color: 'var(--color-ink)' }} />
            <h2 style={{ margin: 0, fontSize: 'var(--text-lg)', fontWeight: 700, color: 'var(--color-ink)', fontFamily: 'var(--font-display)' }}>
              Router Playground
            </h2>
          </div>
          <div style={{ fontSize: 'var(--text-sm)', color: 'var(--color-muted)' }}>
            Stream SSE chat through the CULI router with any model.
          </div>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-xs)' }}>
          <button
            onClick={handleClear}
            title="Clear chat"
            style={{
              display: 'inline-flex', alignItems: 'center', gap: 'var(--space-2xs)',
              padding: 'var(--space-xs) var(--space-sm)', borderRadius: 'var(--radius-md)',
              border: 'var(--rule)', background: 'var(--color-paper-2)', color: 'var(--color-ink-2)',
              fontSize: 'var(--text-sm)', cursor: 'pointer',
            }}
          >
            <Trash size={14} /> Clear
          </button>
        </div>
      </div>

      <div style={{
        display: 'grid', gap: 'var(--space-md)',
        gridTemplateColumns: 'minmax(0, 1fr)',
        gridTemplateRows: 'auto auto 1fr',
        flex: '1 1 auto', minHeight: 0,
      }}>
        {/* Controls */}
        <div style={{
          display: 'grid', gap: 'var(--space-sm)', padding: 'var(--space-md)',
          background: 'var(--color-surface)', borderRadius: 'var(--radius-lg)', border: 'var(--rule)',
          gridTemplateColumns: 'minmax(0, 2fr) 1fr 1fr',
        }}>
          <div style={{ position: 'relative', display: 'flex', flexDirection: 'column', gap: 'var(--space-2xs)' }}>
            <label style={{ fontSize: 'var(--text-xs)', fontWeight: 700, color: 'var(--color-muted)', letterSpacing: '0.06em', textTransform: 'uppercase' }}>
              Model
            </label>
            <div style={{ position: 'relative' }}>
              <button
                className="ra-control-btn"
                onClick={() => setDropdownOpen(o => !o)}
                onBlur={() => window.setTimeout(() => setDropdownOpen(false), 140)}
                style={{
                  width: '100%', display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 'var(--space-xs)',
                  padding: 'var(--space-xs) var(--space-sm)', borderRadius: 'var(--radius-md)',
                  border: 'var(--rule)', background: 'var(--color-paper-2)', color: 'var(--color-ink)',
                  fontSize: 'var(--text-sm)', textAlign: 'left', cursor: 'pointer',
                }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-xs)', minWidth: 0 }}>
                  <ProviderLogo provider={selectedModelObj?.provider} modelName={selectedModel} displayName={selectedModelObj?.display_name || selectedModel} size={20} />
                  <span style={{ fontWeight: 600, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                    {selectedModelObj?.display_name || selectedModel || 'Select model…'}
                  </span>
                  {selectedModelObj && (
                    <span style={{ fontFamily: 'var(--font-mono)', fontSize: 'var(--text-xs)', color: 'var(--color-muted)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', maxWidth: '40%' }}>
                      {selectedModelObj.id}
                    </span>
                  )}
                </div>
                <ChevronDown size={14} style={{ color: 'var(--color-muted)', flexShrink: 0, transform: dropdownOpen ? 'rotate(180deg)' : undefined, transition: `transform var(--dur-fast) var(--ease-out)` }} />
              </button>
              {dropdownOpen && (
                <div style={{
                  position: 'absolute', left: 0, right: 0, top: 'calc(100% + 4px)', zIndex: 'var(--z-dropdown)',
                  background: 'var(--color-paper-2)', border: 'var(--rule)', borderRadius: 'var(--radius-md)',
                  boxShadow: 'var(--shadow-lg)', maxHeight: '320px', overflow: 'hidden',
                  display: 'flex', flexDirection: 'column',
                }}>
                  <div style={{ padding: 'var(--space-xs) var(--space-sm)', borderBottom: 'var(--rule)' }}>
                    <input
                      className="ra-control-input"
                      value={modelSearch}
                      onChange={(e) => setModelSearch(e.target.value)}
                      placeholder="Search models…"
                      autoFocus
                      style={{
                        width: '100%', padding: 'var(--space-2xs) var(--space-xs)',
                        borderRadius: 'var(--radius-sm)', border: 'var(--rule)',
                        background: 'var(--color-paper)', color: 'var(--color-ink)',
                        fontSize: 'var(--text-sm)', outline: 'none',
                      }}
                    />
                  </div>
                  <div style={{ overflowY: 'auto', flex: '1 1 auto' }}>
                    {filteredModelList.length === 0 && (
                      <div style={{ padding: 'var(--space-md)', color: 'var(--color-muted)', fontSize: 'var(--text-sm)', textAlign: 'center' }}>No matches</div>
                    )}
                    {filteredModelList.map(m => (
                      <button
                        key={m.id}
                        onMouseDown={(e) => { e.preventDefault(); setSelectedModel(m.id); setDropdownOpen(false); setModelSearch(''); }}
                        style={{
                          width: '100%', display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 'var(--space-xs)',
                          padding: 'var(--space-xs) var(--space-sm)', textAlign: 'left',
                          background: selectedModel === m.id ? 'var(--color-paper-2)' : 'transparent',
                          color: 'var(--color-ink)', border: 'none', cursor: 'pointer',
                          fontSize: 'var(--text-sm)', borderBottom: '1px solid transparent',
                        }}
                      >
                        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-xs)', minWidth: 0, flex: 1 }}>
                          <ProviderLogo provider={m.provider} modelName={m.id} displayName={m.display_name || m.id} size={18} />
                          <div style={{ display: 'flex', flexDirection: 'column', minWidth: 0 }}>
                            <span style={{ fontWeight: 600 }}>{m.display_name || m.id}</span>
                            <span style={{ fontFamily: 'var(--font-mono)', fontSize: 'var(--text-xs)', color: 'var(--color-muted)' }}>
                              {m.id}
                            </span>
                          </div>
                        </div>
                        <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-neutral)' }}>{m.owned_by}</span>
                      </button>
                    ))}
                  </div>
                </div>
              )}
            </div>
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-2xs)' }}>
            <label style={{ fontSize: 'var(--text-xs)', fontWeight: 700, color: 'var(--color-muted)', letterSpacing: '0.06em', textTransform: 'uppercase', display: 'flex', justifyContent: 'space-between' }}>
              <span>Temperature</span>
              <span style={{ fontFamily: 'var(--font-mono)', color: 'var(--color-ink-2)' }}>{temperature.toFixed(2)}</span>
            </label>
            <input
              type="range"
              min={0} max={1} step={0.01}
              value={temperature}
              onChange={(e) => setTemperature(parseFloat(e.target.value))}
              style={{ width: '100%', accentColor: 'var(--color-ink)' }}
            />
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-2xs)' }}>
            <label style={{ fontSize: 'var(--text-xs)', fontWeight: 700, color: 'var(--color-muted)', letterSpacing: '0.06em', textTransform: 'uppercase' }}>
              Max Tokens
            </label>
            <input
              type="number"
              min={1} max={16384} step={1}
              value={maxTokens}
              onChange={(e) => setMaxTokens(Math.max(1, Math.min(16384, parseInt(e.target.value || '1', 10) || 1)))}
              style={{
                width: '100%', padding: 'var(--space-xs) var(--space-sm)',
                borderRadius: 'var(--radius-md)', border: 'var(--rule)',
                background: 'var(--color-paper-2)', color: 'var(--color-ink)',
                fontSize: 'var(--text-sm)', fontFamily: 'var(--font-mono)', outline: 'none',
              }}
            />
          </div>
        </div>

        {/* Prompts */}
        <div style={{ display: 'grid', gap: 'var(--space-sm)', gridTemplateColumns: '1fr 1fr' }}>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-2xs)' }}>
            <label style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2xs)', fontSize: 'var(--text-xs)', fontWeight: 700, color: 'var(--color-muted)', letterSpacing: '0.06em', textTransform: 'uppercase' }}>
              <Bot size={12} /> System Prompt
            </label>
            <textarea
              value={systemPrompt}
              onChange={(e) => setSystemPrompt(e.target.value)}
              rows={5}
              placeholder="Instructions for the assistant…"
              style={{
                width: '100%', boxSizing: 'border-box', resize: 'vertical',
                padding: 'var(--space-sm)', borderRadius: 'var(--radius-md)',
                border: 'var(--rule)', background: 'var(--color-paper-2)', color: 'var(--color-ink)',
                fontSize: 'var(--text-sm)', fontFamily: 'var(--font-body)', lineHeight: 'var(--leading-normal)',
                outline: 'none',
              }}
            />
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-2xs)' }}>
            <label style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2xs)', fontSize: 'var(--text-xs)', fontWeight: 700, color: 'var(--color-muted)', letterSpacing: '0.06em', textTransform: 'uppercase' }}>
              <User size={12} /> User Message
            </label>
            <textarea
              value={userMessage}
              onChange={(e) => setUserMessage(e.target.value)}
              onKeyDown={(e) => {
                if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') { e.preventDefault(); handleSend(); }
              }}
              rows={5}
              placeholder="Type a message…  (Ctrl/Cmd + Enter to send)"
              style={{
                width: '100%', boxSizing: 'border-box', resize: 'vertical',
                padding: 'var(--space-sm)', borderRadius: 'var(--radius-md)',
                border: 'var(--rule)', background: 'var(--color-paper-2)', color: 'var(--color-ink)',
                fontSize: 'var(--text-sm)', fontFamily: 'var(--font-body)', lineHeight: 'var(--leading-normal)',
                outline: 'none',
              }}
            />
          </div>
        </div>

        {/* Output */}
        <div style={{
          display: 'flex', flexDirection: 'column', gap: 'var(--space-sm)',
          padding: 'var(--space-md)', background: 'var(--color-surface)',
          borderRadius: 'var(--radius-lg)', border: 'var(--rule)',
          minHeight: 0, overflow: 'hidden',
        }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 'var(--space-sm)', flexWrap: 'wrap' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-xs)', flexWrap: 'wrap' }}>
              {meta && (
                <>
                  {meta.provider && (
                    <span style={{
                      display: 'inline-flex', alignItems: 'center', gap: 'var(--space-2xs)',
                      padding: '2px 10px', borderRadius: 'var(--radius-full)',
                      background: 'var(--color-surface)', color: 'var(--color-ink-2)',
                      border: '1px solid var(--color-rule)',
                      fontSize: 'var(--text-xs)', fontWeight: 700, letterSpacing: '0.04em',
                    }}>
                      <ProviderLogo provider={meta.provider} displayName={meta.provider} size={12} />
                      X-Culi-Provider: {meta.provider}
                    </span>
                  )}
                  {meta.model && (
                    <span style={{
                      display: 'inline-flex', alignItems: 'center', gap: 'var(--space-2xs)',
                      padding: '2px 10px', borderRadius: 'var(--radius-full)',
                      background: 'var(--color-surface)', color: 'var(--color-ink-2)',
                      border: '1px solid var(--color-rule)',
                      fontSize: 'var(--text-xs)', fontWeight: 700, letterSpacing: '0.04em',
                    }}>
                      <ProviderLogo modelName={meta.model} displayName={meta.model} size={12} />
                      X-Culi-Model: {meta.model}
                    </span>
                  )}
                </>
              )}
              {loading && (
                <span style={{
                  display: 'inline-flex', alignItems: 'center', gap: 'var(--space-2xs)',
                  padding: '2px 10px', borderRadius: 'var(--radius-full)',
                  background: 'var(--color-surface)', color: 'var(--color-ink-2)',
                  border: '1px solid var(--color-rule)',
                  fontSize: 'var(--text-xs)', fontWeight: 700,
                }}>
                  <RefreshCw size={10} style={{ animation: 'spin 1s linear infinite' }} /> Streaming…
                </span>
              )}
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-xs)' }}>
              <button
                onClick={handleCopy}
                disabled={!assistantOutput}
                title="Copy output"
                style={{
                  display: 'inline-flex', alignItems: 'center', gap: 'var(--space-2xs)',
                  padding: 'var(--space-2xs) var(--space-sm)', borderRadius: 'var(--radius-md)',
                  border: 'var(--rule)', background: copied ? 'var(--color-ink)' : 'var(--color-paper-2)',
                  color: copied ? 'var(--color-paper-1)' : 'var(--color-ink-2)',
                  fontSize: 'var(--text-sm)', cursor: !assistantOutput ? 'not-allowed' : 'pointer',
                  opacity: !assistantOutput ? 0.5 : 1,
                  transition: `all var(--dur-fast) var(--ease-out)`,
                }}
              >
                {copied ? <Check size={14} /> : <Copy size={14} />}
                {copied ? 'Copied' : 'Copy'}
              </button>
              <button
                onClick={handleSend}
                disabled={loading || !selectedModel || !userMessage.trim()}
                style={{
                  display: 'inline-flex', alignItems: 'center', gap: 'var(--space-2xs)',
                  padding: 'var(--space-2xs) var(--space-md)', borderRadius: 'var(--radius-md)',
                  border: '1px solid var(--color-ink)',
                  background: loading ? 'var(--color-ink-2)' : 'var(--color-ink)',
                  color: 'var(--color-paper-1)',
                  fontSize: 'var(--text-sm)', fontWeight: 700,
                  cursor: (loading || !selectedModel || !userMessage.trim()) ? 'not-allowed' : 'pointer',
                  opacity: (loading || !selectedModel || !userMessage.trim()) ? 0.7 : 1,
                  boxShadow: !loading ? 'var(--shadow-md)' : 'none',
                  transition: `all var(--dur-fast) var(--ease-out)`,
                }}
              >
                {loading ? <RefreshCw size={14} style={{ animation: 'spin 1s linear infinite' }} /> : <Send size={14} />}
                {loading ? 'Sending…' : 'Send'}
              </button>
            </div>
          </div>

          <div
            ref={outputRef}
            style={{
              flex: '1 1 auto', minHeight: 0, overflowY: 'auto',
              padding: 'var(--space-md)', borderRadius: 'var(--radius-md)',
              background: 'var(--color-paper)', border: 'var(--rule)',
              fontSize: 'var(--text-sm)', color: 'var(--color-ink)',
              fontFamily: 'var(--font-body)', lineHeight: 'var(--leading-relaxed)',
              whiteSpace: 'pre-wrap', wordBreak: 'break-word',
            }}
          >
            {error && (
              <div style={{
                marginBottom: 'var(--space-sm)', padding: 'var(--space-sm)',
                borderRadius: 'var(--radius-md)',
                border: '1px solid var(--color-rule)',
                background: 'var(--color-surface)',
                color: 'var(--color-ink-2)', fontSize: 'var(--text-sm)',
              }}>
                {error}
              </div>
            )}

            {!displayedOutput && !loading && !error && (
              <div style={{ color: 'var(--color-muted)', textAlign: 'center', padding: 'var(--space-xl) 0' }}>
                <Sparkles size={28} style={{ opacity: 0.5, marginBottom: 'var(--space-sm)' }} />
                <div style={{ fontWeight: 600, color: 'var(--color-ink-2)' }}>No response yet</div>
                <div style={{ fontSize: 'var(--text-sm)', marginTop: 'var(--space-2xs)' }}>
                  Fill in the user message above and click <strong>Send</strong>.
                </div>
              </div>
            )}

            {loading && !displayedOutput && (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-xs)' }}>
                {Array.from({ length: 5 }).map((_, i) => (
                  <div
                    key={i}
                    style={{
                      height: '14px',
                      borderRadius: 'var(--radius-sm)',
                      background: 'var(--color-rule)',
                      width: `${40 + ((i * 17) % 55)}%`,
                      animation: 'pulse 1.4s ease-in-out infinite',
                      animationDelay: `${i * 120}ms`,
                    }}
                  />
                ))}
              </div>
            )}

            <div>
              {displayedOutput}
              {loading && displayedOutput && (
                <span style={{
                  display: 'inline-block', width: '8px', height: '1em',
                  verticalAlign: '-0.15em', marginLeft: '2px',
                  background: 'var(--color-ink)', borderRadius: '1px',
                  animation: 'blink 1s step-end infinite',
                }} />
              )}
            </div>
          </div>

          <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 'var(--text-xs)', color: 'var(--color-muted)' }}>
            <div>
              Output tokens (est.): <span style={{ fontFamily: 'var(--font-mono)', color: 'var(--color-ink-2)' }}>{Math.ceil(assistantOutput.length / 4)}</span>
            </div>
            <div>
              <kbd style={{
                padding: '1px 6px', borderRadius: 'var(--radius-sm)',
                border: 'var(--rule)', background: 'var(--color-paper-2)',
                fontFamily: 'var(--font-mono)', fontSize: '10px',
              }}>Ctrl</kbd> + <kbd style={{
                padding: '1px 6px', borderRadius: 'var(--radius-sm)',
                border: 'var(--rule)', background: 'var(--color-paper-2)',
                fontFamily: 'var(--font-mono)', fontSize: '10px',
              }}>Enter</kbd> to send
            </div>
          </div>
        </div>
      </div>

      <style>{`
        @keyframes pulse {
          0%, 100% { opacity: 0.45; }
          50% { opacity: 0.9; }
        }
        @keyframes spin {
          from { transform: rotate(0deg); }
          to { transform: rotate(360deg); }
        }
        @keyframes blink {
          0%, 100% { opacity: 1; }
          50% { opacity: 0; }
        }
      `}</style>
    </div>
  );
}
