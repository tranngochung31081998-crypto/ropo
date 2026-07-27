import React, { useState, useEffect, useMemo } from 'react';
import { Search, RefreshCw, Cpu, Sparkles } from 'lucide-react';
import { getRouterModels, type RouterModel } from '../../api/routerClient';
import { useRouterStore } from './store';
import { ProviderLogo } from './providerLogos';

const QVERIS_MODELS: RouterModel[] = [
  { id: 'deepseek-r1', object: 'model', created: 1730000000, owned_by: 'deepseek', display_name: 'DeepSeek R1', description: 'Reasoning model with advanced chain-of-thought capabilities.', provider: 'qveris' },
  { id: 'deepseek-v3.2', object: 'model', created: 1730000001, owned_by: 'deepseek', display_name: 'DeepSeek V3.2', description: 'Latest general-purpose DeepSeek model with strong coding.', provider: 'qveris' },
  { id: 'gpt-5.2', object: 'model', created: 1730000002, owned_by: 'openai', display_name: 'GPT-5.2', description: 'Cutting-edge OpenAI flagship, multimodal and reasoning.', provider: 'qveris' },
  { id: 'gpt-4.1', object: 'model', created: 1730000003, owned_by: 'openai', display_name: 'GPT-4.1', description: 'Stable workhorse OpenAI model for general tasks.', provider: 'qveris' },
  { id: 'claude-3-7', object: 'model', created: 1730000004, owned_by: 'anthropic', display_name: 'Claude 3.7 Sonnet', description: 'Balanced Anthropic model with extended thinking.', provider: 'qveris' },
  { id: 'claude-opus-4-5', object: 'model', created: 1730000005, owned_by: 'anthropic', display_name: 'Claude Opus 4.5', description: 'Anthropic top-tier reasoning and long context.', provider: 'qveris' },
  { id: 'gemini-2.5-flash-image', object: 'model', created: 1730000006, owned_by: 'google', display_name: 'Gemini 2.5 Flash Image', description: 'Fast Google multimodal with strong vision.', provider: 'qveris' },
  { id: 'gemini-3-pro-image-preview', object: 'model', created: 1730000007, owned_by: 'google', display_name: 'Gemini 3 Pro Image Preview', description: 'Preview of next-gen Gemini vision model.', provider: 'qveris' },
  { id: 'openai/gpt-5.6-luna', object: 'model', created: 1730000008, owned_by: 'openrouter', display_name: 'GPT-5.6 Luna (OpenRouter)', description: 'Experimental Luna variant via OpenRouter.', provider: 'qveris' },
  { id: 'openai/gpt-5.6-terra', object: 'model', created: 1730000009, owned_by: 'openrouter', display_name: 'GPT-5.6 Terra (OpenRouter)', description: 'Experimental Terra variant via OpenRouter.', provider: 'qveris' },
  { id: 'openai/gpt-5.6-sol', object: 'model', created: 1730000010, owned_by: 'openrouter', display_name: 'GPT-5.6 Sol (OpenRouter)', description: 'Experimental Sol variant via OpenRouter.', provider: 'qveris' },
  { id: 'anthropic/claude-opus-4.8', object: 'model', created: 1730000011, owned_by: 'openrouter', display_name: 'Claude Opus 4.8 (OpenRouter)', description: 'Latest Opus via OpenRouter aggregation.', provider: 'qveris' },
  { id: 'anthropic/claude-sonnet-5', object: 'model', created: 1730000012, owned_by: 'openrouter', display_name: 'Claude Sonnet 5 (OpenRouter)', description: 'Sonnet 5 speed + intelligence balance.', provider: 'qveris' },
  { id: 'anthropic/claude-fable-5', object: 'model', created: 1730000013, owned_by: 'openrouter', display_name: 'Claude Fable 5 (OpenRouter)', description: 'Creative long-form Fable variant.', provider: 'qveris' },
  { id: 'deepseek/deepseek-v4-pro', object: 'model', created: 1730000014, owned_by: 'openrouter', display_name: 'DeepSeek V4 Pro (OpenRouter)', description: 'DeepSeek V4 Pro via OpenRouter.', provider: 'qveris' },
  { id: 'deepseek/deepseek-v4-flash', object: 'model', created: 1730000015, owned_by: 'openrouter', display_name: 'DeepSeek V4 Flash (OpenRouter)', description: 'Fast and affordable V4 Flash.', provider: 'qveris' },
  { id: 'google/gemini-3.1-pro-preview', object: 'model', created: 1730000016, owned_by: 'openrouter', display_name: 'Gemini 3.1 Pro Preview (OpenRouter)', description: 'Next Gemini pro preview via OpenRouter.', provider: 'qveris' },
  { id: 'google/gemini-3.1-flash-lite', object: 'model', created: 1730000017, owned_by: 'openrouter', display_name: 'Gemini 3.1 Flash Lite (OpenRouter)', description: 'Ultra-fast lightweight Gemini.', provider: 'qveris' },
  { id: 'x-ai/grok-4.5', object: 'model', created: 1730000018, owned_by: 'openrouter', display_name: 'Grok 4.5 (OpenRouter)', description: 'xAI latest reasoning Grok model.', provider: 'qveris' },
  { id: 'qwen/qwen3.7-plus', object: 'model', created: 1730000019, owned_by: 'openrouter', display_name: 'Qwen 3.7 Plus (OpenRouter)', description: 'Alibaba Qwen 3.7 plus variant.', provider: 'qveris' },
  { id: 'moonshotai/kimi-k3', object: 'model', created: 1730000020, owned_by: 'openrouter', display_name: 'Kimi K3 (OpenRouter)', description: 'Moonshot K3 long-context model.', provider: 'qveris' },
  { id: 'moonshotai/kimi-k2.6', object: 'model', created: 1730000021, owned_by: 'openrouter', display_name: 'Kimi K2.6 (OpenRouter)', description: 'Kimi K2.6 stable release.', provider: 'qveris' },
  { id: 'moonshotai/kimi-k2.7-code', object: 'model', created: 1730000022, owned_by: 'openrouter', display_name: 'Kimi K2.7 Code (OpenRouter)', description: 'Code-specialized Kimi variant.', provider: 'qveris' },
  { id: 'z-ai/glm-5.2', object: 'model', created: 1730000023, owned_by: 'openrouter', display_name: 'GLM 5.2 (OpenRouter)', description: 'Zhipu GLM 5.2 latest general model.', provider: 'qveris' },
  { id: 'minimax/minimax-m3', object: 'model', created: 1730000024, owned_by: 'openrouter', display_name: 'MiniMax M3 (OpenRouter)', description: 'MiniMax M3 reasoning and multimodal.', provider: 'qveris' },
  { id: 'xiaomi/mimo-v2.5-pro', object: 'model', created: 1730000025, owned_by: 'openrouter', display_name: 'Mimo V2.5 Pro (OpenRouter)', description: 'Xiaomi Mimo V2.5 Pro model.', provider: 'qveris' },
];

const FREEMODEL_SEED: RouterModel[] = [
  { id: 'culi-deepseek-r1-free', object: 'model', created: 1730000100, owned_by: 'blackbox', display_name: 'DeepSeek R1 (Free)', description: 'Free-tier R1 via CULI freemodel pool.', provider: 'blackbox' },
  { id: 'culi-gpt-4o-mini-free', object: 'model', created: 1730000101, owned_by: 'sixth', display_name: 'GPT-4o Mini (Free)', description: 'Free GPT-4o Mini via sixth-sense accounts.', provider: 'sixth' },
  { id: 'culi-claude-sonnet-free', object: 'model', created: 1730000102, owned_by: 'blackbox', display_name: 'Claude Sonnet (Free)', description: 'Free Claude Sonnet via freemodel rotation.', provider: 'blackbox' },
  { id: 'culi-gemini-flash-free', object: 'model', created: 1730000103, owned_by: 'sixth', display_name: 'Gemini Flash (Free)', description: 'Free Gemini Flash via sixth-sense pool.', provider: 'sixth' },
];

type FilterKey = 'all' | 'culi-freemodel' | 'qveris-wangsu' | 'qveris-openrouter' | 'custom';

const FILTER_PILLS: { key: FilterKey; label: string }[] = [
  { key: 'all', label: 'All' },
  { key: 'culi-freemodel', label: 'CULI Freemodel' },
  { key: 'qveris-wangsu', label: 'QVERIS Wangsu' },
  { key: 'qveris-openrouter', label: 'QVERIS OpenRouter' },
  { key: 'custom', label: 'Custom' },
];

function providerBadgeColor(prov: string): { bg: string; ink: string; rule: string; label: string } {
  switch (prov) {
    case 'qveris':
      return { bg: 'var(--color-surface)', ink: 'var(--color-ink-2)', rule: 'var(--color-rule)', label: 'QVERIS' };
    case 'blackbox':
      return { bg: 'var(--color-surface)', ink: 'var(--color-ink-2)', rule: 'var(--color-rule)', label: 'BLACKBOX' };
    case 'sixth':
      return { bg: 'var(--color-surface)', ink: 'var(--color-ink-2)', rule: 'var(--color-rule)', label: 'SIXTH' };
    case 'custom':
    default:
      return { bg: 'var(--color-surface)', ink: 'var(--color-ink-2)', rule: 'var(--color-rule)', label: prov.toUpperCase() };
  }
}

export default function ModelCatalogPanel() {
  const { customProviders } = useRouterStore();
  const [models, setModels] = useState<RouterModel[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState('');
  const [filter, setFilter] = useState<FilterKey>('all');
  const [grouped, setGrouped] = useState(true);

  useEffect(() => {
    let cancelled = false;
    async function load() {
      setLoading(true);
      setError(null);
      try {
        const res = await getRouterModels();
        if (!cancelled) setModels(res.data || []);
      } catch (e) {
        if (!cancelled) {
          setError(e instanceof Error ? e.message : 'Failed to load models');
          const customFromStore: RouterModel[] = customProviders.flatMap(p =>
            p.models.map(m => ({
              id: m,
              object: 'model' as const,
              created: Date.parse(p.createdAt) / 1000,
              owned_by: p.providerKey,
              display_name: m.includes('/') ? m.split('/').pop() || m : m,
              description: `Model offered by ${p.displayName}.`,
              provider: 'qveris',
            } as RouterModel))
          );
          customFromStore.forEach(m => ((m as unknown) as { provider: string }).provider = 'custom');
          setModels([...FREEMODEL_SEED, ...QVERIS_MODELS, ...customFromStore]);
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    load();
    return () => { cancelled = true; };
  }, [customProviders]);

  function refresh() {
    setLoading(true);
    setError(null);
    getRouterModels()
      .then(r => setModels(r.data || []))
      .catch(e => {
        setError(e instanceof Error ? e.message : 'Failed to load');
      })
      .finally(() => setLoading(false));
  }

  const filtered = useMemo(() => {
    const s = search.trim().toLowerCase();
    return models.filter(m => {
      const hitSearch = !s || m.id.toLowerCase().includes(s) || m.display_name.toLowerCase().includes(s) || m.owned_by.toLowerCase().includes(s);
      if (!hitSearch) return false;
      const prov = m.provider as string;
      switch (filter) {
        case 'all': return true;
        case 'culi-freemodel': return prov === 'blackbox' || prov === 'sixth';
        case 'qveris-wangsu': return prov === 'qveris' && !m.id.includes('/');
        case 'qveris-openrouter': return prov === 'qveris' && m.id.includes('/');
        case 'custom': return prov === 'custom';
        default: return true;
      }
    });
  }, [models, search, filter]);

  const groupedData = useMemo(() => {
    if (!grouped) return null;
    const g: Record<string, RouterModel[]> = {};
    for (const m of filtered) {
      const key = m.provider || 'other';
      (g[key] ||= []).push(m);
    }
    return g;
  }, [filtered, grouped]);

  const cardStyle: React.CSSProperties = {
    display: 'flex', flexDirection: 'column', gap: 'var(--space-xs)',
    padding: 'var(--space-md)', borderRadius: 'var(--radius-lg)',
    background: 'var(--color-surface)', border: '1px solid var(--color-rule)',
    transition: `all var(--dur-fast) var(--ease-out)`,
    cursor: 'pointer', minHeight: '130px',
  };

  function ModelCard({ m }: { m: RouterModel }) {
    const [hovered, setHovered] = useState(false);
    const badge = providerBadgeColor(m.provider);
    return (
      <div
        style={{
          ...cardStyle,
          boxShadow: hovered ? 'var(--shadow-md)' : 'var(--shadow-sm)',
          transform: hovered ? 'translateY(-2px)' : 'none',
          borderColor: hovered ? 'var(--color-ink-2)' : 'var(--color-rule)',
        }}
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => setHovered(false)}
      >
        <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', gap: 'var(--space-xs)' }}>
          <ProviderLogo provider={m.provider} modelName={m.id} displayName={m.display_name} size={22} title={`${m.display_name} (${m.provider})`} />
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 'var(--space-xs)', flex: 1 }}>
            <span style={{
              display: 'inline-flex', alignItems: 'center', gap: 'var(--space-2xs)',
              padding: '2px 8px', borderRadius: 'var(--radius-full)',
              fontSize: 'var(--text-xs)', fontWeight: 700, letterSpacing: '0.04em',
              background: badge.bg, color: badge.ink, border: '1px solid ' + badge.rule,
            }}>
              {badge.label}
            </span>
            <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-muted)', fontFamily: 'var(--font-mono)' }}>
              {m.owned_by}
            </span>
          </div>
        </div>
        <div style={{ fontSize: 'var(--text-md)', fontWeight: 600, color: 'var(--color-ink)', lineHeight: 'var(--leading-tight)' }}>
          {m.display_name}
        </div>
        <div style={{ fontSize: 'var(--text-xs)', color: 'var(--color-muted)', fontFamily: 'var(--font-mono)', wordBreak: 'break-all' }}>
          {m.id}
        </div>
        <div style={{ fontSize: 'var(--text-sm)', color: 'var(--color-ink-2)', lineHeight: 'var(--leading-snug)', marginTop: 'auto' }}>
          {m.description}
        </div>
      </div>
    );
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-md)', height: '100%', minHeight: 0 }}>
      <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', gap: 'var(--space-md)', flexWrap: 'wrap' }}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-2xs)' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-xs)' }}>
            <Sparkles size={18} style={{ color: 'var(--color-ink)' }} />
            <h2 style={{ margin: 0, fontSize: 'var(--text-lg)', fontWeight: 700, color: 'var(--color-ink)', fontFamily: 'var(--font-display)' }}>
              Model Catalog
            </h2>
            <div style={{ display: 'inline-flex', alignItems: 'center', gap: '4px' }}>
              <ProviderLogo provider="openai" size={18} />
              <ProviderLogo provider="anthropic" size={18} />
              <ProviderLogo provider="google" size={18} />
              <ProviderLogo provider="deepseek" size={18} />
            </div>
          </div>
          <div style={{ fontSize: 'var(--text-sm)', color: 'var(--color-muted)' }}>
            {loading ? 'Loading models…' : `${filtered.length} of ${models.length} models`}
            {error && <span style={{ color: 'var(--color-muted)', marginLeft: 'var(--space-xs)' }}>· using mock fallback</span>}
          </div>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-xs)' }}>
          <button
            className="ra-control-btn"
            onClick={refresh}
            disabled={loading}
            title="Refresh"
            style={{
              display: 'inline-flex', alignItems: 'center', gap: 'var(--space-2xs)',
              padding: 'var(--space-xs) var(--space-sm)', borderRadius: 'var(--radius-md)',
              border: '1px solid var(--color-rule)', background: 'var(--color-paper-2)', color: 'var(--color-ink-2)',
              fontSize: 'var(--text-sm)', cursor: loading ? 'wait' : 'pointer',
            }}
          >
            <RefreshCw size={14} style={{ animation: loading ? 'spin 1s linear infinite' : undefined }} />
            Refresh
          </button>
          <button
            className="ra-control-btn"
            onClick={() => setGrouped(g => !g)}
            style={{
              display: 'inline-flex', alignItems: 'center', gap: 'var(--space-2xs)',
              padding: 'var(--space-xs) var(--space-sm)', borderRadius: 'var(--radius-md)',
              border: '1px solid',
              borderColor: grouped ? 'var(--color-ink)' : 'var(--color-rule)',
              background: grouped ? 'var(--color-surface)' : 'var(--color-paper-2)',
              color: grouped ? 'var(--color-ink)' : 'var(--color-ink-2)',
              fontSize: 'var(--text-sm)', cursor: 'pointer',
            }}
          >
            {grouped ? 'Grouped' : 'Flat'}
          </button>
        </div>
      </div>

      <div style={{ display: 'flex', gap: 'var(--space-sm)', alignItems: 'center', flexWrap: 'wrap' }}>
        <div style={{
          display: 'flex', alignItems: 'center', gap: 'var(--space-xs)',
          flex: '1 1 260px', minWidth: '200px',
          padding: 'var(--space-xs) var(--space-sm)', borderRadius: 'var(--radius-md)',
          border: '1px solid var(--color-rule)', background: 'var(--color-paper-2)',
        }}>
          <Search size={14} style={{ color: 'var(--color-muted)', flexShrink: 0 }} />
          <input
            className="ra-control-input"
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search by name, id, or owner…"
            style={{
              flex: 1, border: 'none', outline: 'none', background: 'transparent',
              color: 'var(--color-ink)', fontSize: 'var(--text-sm)', minWidth: 0,
            }}
          />
        </div>
        <div style={{ display: 'flex', gap: 'var(--space-2xs)', flexWrap: 'wrap' }}>
          {FILTER_PILLS.map(p => {
            const active = filter === p.key;
            return (
              <button
                key={p.key}
                onClick={() => setFilter(p.key)}
                style={{
                  display: 'inline-flex', alignItems: 'center', gap: 'var(--space-2xs)',
                  padding: 'var(--space-xs) var(--space-md)', borderRadius: 'var(--radius-full)',
                  minHeight: 30,
                  border: '1px solid',
                  borderColor: active ? 'var(--color-ink)' : 'var(--color-rule)',
                  background: active ? 'var(--color-surface)' : 'var(--color-paper-2)',
                  color: active ? 'var(--color-ink)' : 'var(--color-ink-2)',
                  fontSize: 'var(--text-sm)', fontWeight: active ? 600 : 500,
                  cursor: 'pointer', transition: `all var(--dur-fast) var(--ease-out)`,
                }}
              >
                {p.key !== 'all' && <ProviderLogo provider={p.label} displayName={p.label} size={14} />}
                {p.label}
              </button>
            );
          })}
        </div>
      </div>

      <div style={{
        flex: '1 1 auto', overflowY: 'auto', paddingRight: 'var(--space-xs)',
        minHeight: 0, display: 'flex', flexDirection: 'column', gap: 'var(--space-lg)',
      }}>
        {loading && (
          <div style={{
            display: 'grid', gap: 'var(--space-sm)',
            gridTemplateColumns: 'repeat(auto-fill, minmax(260px, 1fr))',
          }}>
            {Array.from({ length: 8 }).map((_, i) => (
              <div key={i} style={{
                ...cardStyle, background: 'var(--color-paper-2)',
                animation: 'pulse 1.6s ease-in-out infinite',
                animationDelay: `${i * 80}ms`,
              }}>
                <div style={{ height: '18px', width: '80px', borderRadius: 'var(--radius-full)', background: 'var(--color-rule)' }} />
                <div style={{ height: '22px', width: '70%', borderRadius: 'var(--radius-sm)', background: 'var(--color-rule)' }} />
                <div style={{ height: '14px', width: '90%', borderRadius: 'var(--radius-sm)', background: 'var(--color-rule)' }} />
                <div style={{ height: '16px', width: '100%', borderRadius: 'var(--radius-sm)', background: 'var(--color-rule)' }} />
                <div style={{ height: '16px', width: '60%', borderRadius: 'var(--radius-sm)', background: 'var(--color-rule)' }} />
              </div>
            ))}
          </div>
        )}

        {!loading && groupedData && (
          Object.entries(groupedData).map(([groupKey, items]) => {
            const badge = providerBadgeColor(groupKey);
            return (
              <section key={groupKey} style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-sm)' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-xs)' }}>
                  <h3 style={{
                    margin: 0, fontSize: 'var(--text-md)', fontWeight: 700, color: 'var(--color-ink)',
                    fontFamily: 'var(--font-display)',
                  }}>
                    {badge.label}
                  </h3>
                  <span style={{
                    fontSize: 'var(--text-xs)', color: badge.ink,
                    background: badge.bg, padding: '2px 8px', borderRadius: 'var(--radius-full)', fontWeight: 600,
                  }}>
                    {items.length}
                  </span>
                  <div style={{ flex: 1, height: '1px', background: 'var(--color-rule)' }} />
                </div>
                <div style={{
                  display: 'grid', gap: 'var(--space-sm)',
                  gridTemplateColumns: 'repeat(auto-fill, minmax(260px, 1fr))',
                }}>
                  {items.map(m => <ModelCard key={m.id} m={m} />)}
                </div>
              </section>
            );
          })
        )}

        {!loading && !groupedData && (
          <div style={{
            display: 'grid', gap: 'var(--space-sm)',
            gridTemplateColumns: 'repeat(auto-fill, minmax(260px, 1fr))',
          }}>
            {filtered.map(m => <ModelCard key={m.id} m={m} />)}
          </div>
        )}

        {!loading && filtered.length === 0 && (
          <div style={{
            display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center',
            gap: 'var(--space-sm)', padding: 'var(--space-2xl)', color: 'var(--color-muted)',
            textAlign: 'center', border: '1px dashed var(--color-rule)', borderRadius: 'var(--radius-lg)',
          }}>
            <Search size={28} style={{ opacity: 0.5 }} />
            <div style={{ fontSize: 'var(--text-md)', color: 'var(--color-ink-2)', fontWeight: 600 }}>
              No models match your filters
            </div>
            <div style={{ fontSize: 'var(--text-sm)' }}>
              Try clearing the search or switching filters.
            </div>
          </div>
        )}
      </div>

      <style>{`
        @keyframes pulse {
          0%, 100% { opacity: 0.55; }
          50% { opacity: 0.85; }
        }
        @keyframes spin {
          from { transform: rotate(0deg); }
          to { transform: rotate(360deg); }
        }
      `}</style>
    </div>
  );
}
