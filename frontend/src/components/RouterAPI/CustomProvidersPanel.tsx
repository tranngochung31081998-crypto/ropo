import { useRef, useState } from 'react';
import {
  Key, Plus, Trash2, Edit3, Check, ChevronDown, ChevronRight,
  Save, RefreshCw, Server, Copy, Eye, EyeOff, X,
} from 'lucide-react';
import { useRouterStore, type CustomProvider } from './store';
import { ProviderLogo } from './providerLogos';

function maskKey(key: string): string {
  if (!key) return 'N/A';
  if (key.length <= 8) return key.charAt(0) + '\u2026' + key.charAt(key.length - 1);
  const prefix = key.startsWith('sk-') ? key.slice(0, 3) : key.slice(0, 2);
  const suffix = key.slice(-4);
  return `${prefix}\u2026${suffix}`;
}

function ProviderKeysSection({ provider }: { provider: CustomProvider }) {
  const { addKeyToProvider, removeKeyFromProvider, toggleProviderKey } = useRouterStore();
  const [expanded, setExpanded] = useState(provider.keys.length > 0);
  const [labelInput, setLabelInput] = useState('');
  const [keyInput, setKeyInput] = useState('');
  const [showKey, setShowKey] = useState<Record<string, boolean>>({});
  const [copiedId, setCopiedId] = useState<string | null>(null);

  const handleAdd = () => {
    if (!keyInput.trim()) return;
    addKeyToProvider(provider.id, labelInput.trim(), keyInput.trim());
    setLabelInput('');
    setKeyInput('');
  };

  const handleCopy = async (key: string, id: string) => {
    try {
      await navigator.clipboard.writeText(key);
      setCopiedId(id);
      setTimeout(() => setCopiedId(null), 1500);
    } catch {
      // noop
    }
  };

  const activeKeys = provider.keys.filter(k => k.active).length;

  return (
    <div className="cp-ksec">
      <button
        onClick={() => setExpanded(e => !e)}
        className="cp-ksec-toggle"
      >
        {expanded
          ? <ChevronDown size={13} style={{ color: 'var(--color-muted)' }} />
          : <ChevronRight size={13} style={{ color: 'var(--color-muted)' }} />}
        <Key size={12} style={{ color: 'var(--color-ink-2)' }} />
        <span className="cp-ksec-label">Keys</span>
        <span className="cp-ksec-count">
          {provider.keys.length} total &middot; {activeKeys} active
        </span>
      </button>

      {expanded && (
        <div className="cp-ksec-body">
          <div className="cp-addkey-row">
            <div className="r-field">
              <input
                type="text"
                placeholder="Label"
                value={labelInput}
                onChange={e => setLabelInput(e.target.value)}
                onKeyDown={e => { if (e.key === 'Enter') handleAdd(); }}
              />
            </div>
            <div className="r-field">
              <input
                type="password"
                placeholder="sk-..."
                value={keyInput}
                onChange={e => setKeyInput(e.target.value)}
                onKeyDown={e => { if (e.key === 'Enter') handleAdd(); }}
                style={{ fontFamily: "'Geist Mono', ui-monospace, monospace" }}
              />
            </div>
            <button
              onClick={handleAdd}
              disabled={!keyInput.trim()}
              className="r-btn r-btn-primary"
            >
              <Plus size={11} />
              Add
            </button>
          </div>

          {provider.keys.length === 0 ? (
            <div className="cp-keys-empty">
              No keys configured for this provider
            </div>
          ) : (
            <div className="cp-keys-list">
              {provider.keys.map(k => (
                <div key={k.id} className={`cp-krow ${k.active ? '' : 'cp-krow-off'}`}>
                  <div className="cp-krow-label">
                    <ProviderLogo size={16} provider={provider.id} displayName={provider.displayName} />
                    {k.label}
                  </div>
                  <div className="qp-key-cell">
                    <code className="qp-key-mono">
                      {showKey[k.id] ? k.key : maskKey(k.key)}
                    </code>
                    <button
                      className="qp-key-btn"
                      onClick={() => setShowKey(p => ({ ...p, [k.id]: !p[k.id] }))}
                    >
                      {showKey[k.id] ? <EyeOff size={10} /> : <Eye size={10} />}
                    </button>
                    <button
                      className="qp-key-btn"
                      onClick={() => handleCopy(k.key, k.id)}
                    >
                      {copiedId === k.id
                        ? <Check size={10} style={{ color: 'var(--color-ink-2)' }} />
                        : <Copy size={10} />}
                    </button>
                  </div>
                  <div style={{ display: 'flex', justifyContent: 'center' }}>
                    <label className="r-tgl">
                      <input
                        type="checkbox"
                        checked={k.active}
                        onChange={() => toggleProviderKey(provider.id, k.id)}
                      />
                      <span className="r-tgl-track" />
                    </label>
                  </div>
                  <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
                    <button
                      className="r-btn r-btn-icon r-btn-ghost"
                      onClick={() => removeKeyFromProvider(provider.id, k.id)}
                      style={{ color: 'var(--color-ink-2)' }}
                    >
                      <Trash2 size={11} />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function ProviderCard({ provider }: { provider: CustomProvider }) {
  const { updateCustomProvider, removeCustomProvider } = useRouterStore();
  const [editingUrl, setEditingUrl] = useState(false);
  const [urlDraft, setUrlDraft] = useState(provider.baseUrl);
  const [editingModels, setEditingModels] = useState(false);
  const [modelsDraft, setModelsDraft] = useState(provider.models.join(', '));
  const [open, setOpen] = useState(true);

  const commitUrl = () => {
    updateCustomProvider(provider.id, { baseUrl: urlDraft.trim() || provider.baseUrl });
    setEditingUrl(false);
  };

  const commitModels = () => {
    const list = modelsDraft
      .split(',')
      .map(s => s.trim())
      .filter(Boolean);
    updateCustomProvider(provider.id, { models: list.length > 0 ? list : provider.models });
    setEditingModels(false);
  };

  const modelCount = provider.models.length;
  const activeKeys = provider.keys.filter(k => k.active).length;

  return (
    <div className={`cp-card ${provider.active ? '' : 'cp-card-off'}`}>
      <div className="cp-card-head" onClick={() => setOpen(o => !o)}>
        <ProviderLogo provider={provider.id} modelName={provider.baseUrl || provider.displayName} displayName={provider.displayName || provider.id} size={40} title={provider.displayName || provider.id} />
        <div className="cp-info">
          <div className="cp-info-head">
            <div className="cp-name">{provider.displayName}</div>
            {modelCount > 0 && (
              <span className="r-chip">{modelCount} models</span>
            )}
            {activeKeys > 0 && (
              <span className="r-chip">
                <Key size={8} /> {activeKeys}
              </span>
            )}
            <div className="cp-actions">
              <button
                className="r-btn r-btn-icon r-btn-ghost"
                onClick={e => { e.stopPropagation(); removeCustomProvider(provider.id); }}
                title="Remove provider"
              >
                <Trash2 size={12} />
              </button>
              <label className="r-tgl" title={provider.active ? 'Disable' : 'Enable'}>
                <input
                  type="checkbox"
                  checked={provider.active}
                  onChange={e => { e.stopPropagation(); updateCustomProvider(provider.id, { active: !provider.active }); }}
                />
                <span className="r-tgl-track" />
              </label>
              <span className={`cp-chev ${open ? 'cp-chev-open' : ''}`}>
                <ChevronDown size={14} />
              </span>
            </div>
          </div>

          <div className="cp-url" onClick={e => e.stopPropagation()}>
            <div className="r-sec-h" style={{ marginBottom: 4 }}>Base URL</div>
            {editingUrl ? (
              <div className="r-field">
                <Server size={12} className="r-field-ic" />
                <input
                  type="text"
                  value={urlDraft}
                  onChange={e => setUrlDraft(e.target.value)}
                  onKeyDown={e => {
                    if (e.key === 'Enter') commitUrl();
                    if (e.key === 'Escape') { setEditingUrl(false); setUrlDraft(provider.baseUrl); }
                  }}
                  onBlur={commitUrl}
                  autoFocus
                  style={{ fontFamily: "'Geist Mono', ui-monospace, monospace" }}
                />
                <button className="r-btn r-btn-icon" onClick={commitUrl}>
                  <Check size={11} />
                </button>
              </div>
            ) : (
              <button
                className="cp-edit-btn"
                onClick={() => { setUrlDraft(provider.baseUrl); setEditingUrl(true); }}
                title="Click to edit base URL"
              >
                <Server size={12} className="r-field-ic" />
                <code className="cp-url-mono">{provider.baseUrl}</code>
                <Edit3 size={10} style={{ color: 'var(--color-muted)', opacity: 0.6 }} />
              </button>
            )}
          </div>

          <div className="cp-models-section" onClick={e => e.stopPropagation()}>
            <div className="r-sec-h" style={{ marginBottom: 4 }}>Models</div>
            {editingModels ? (
              <div className="r-field" style={{ height: 'auto' }}>
                <Edit3 size={12} className="r-field-ic" />
                <input
                  type="text"
                  value={modelsDraft}
                  onChange={e => setModelsDraft(e.target.value)}
                  onKeyDown={e => {
                    if (e.key === 'Enter') commitModels();
                    if (e.key === 'Escape') { setEditingModels(false); setModelsDraft(provider.models.join(', ')); }
                  }}
                  onBlur={commitModels}
                  placeholder="model-a, model-b, model-c"
                  autoFocus
                  style={{ fontFamily: "'Geist Mono', ui-monospace, monospace" }}
                />
                <button className="r-btn r-btn-icon" onClick={commitModels}>
                  <Save size={11} />
                </button>
              </div>
            ) : (
              <button
                className="cp-edit-btn"
                onClick={() => { setModelsDraft(provider.models.join(', ')); setEditingModels(true); }}
                title="Click to edit models"
              >
                <Edit3 size={12} className="r-field-ic" />
                <div className="cp-model-chips">
                  {provider.models.length === 0 ? (
                    <span className="cp-models-empty">No models &mdash; click to add</span>
                  ) : (
                    provider.models.map((m, i) => (
                      <span key={i} className="r-chip" style={{ fontSize: '10px' }}>
                        {m}
                      </span>
                    ))
                  )}
                </div>
              </button>
            )}
          </div>
        </div>
      </div>

      {open && (
        <div className="cp-card-body">
          <ProviderKeysSection provider={provider} />
        </div>
      )}
    </div>
  );
}

export function CustomProvidersPanel() {
  const { customProviders, addCustomProvider } = useRouterStore();
  const [showForm, setShowForm] = useState(false);
  const [name, setName] = useState('');
  const [baseUrl, setBaseUrl] = useState('');
  const [logoDataUrl, setLogoDataUrl] = useState<string | null>(null);
  const fileRef = useRef<HTMLInputElement>(null);

  const totalProviders = customProviders.length;
  const activeProviders = customProviders.filter(p => p.active).length;
  const totalModels = customProviders.reduce((sum, p) => sum + p.models.length, 0);
  const totalKeys = customProviders.reduce((sum, p) => sum + p.keys.filter(k => k.active).length, 0);

  const handleFile = (file: File | undefined) => {
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => setLogoDataUrl(reader.result as string);
    reader.readAsDataURL(file);
  };

  const handleSubmit = () => {
    if (!name.trim() || !baseUrl.trim()) return;
    const key = name
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '_')
      .replace(/^_|_$/g, '') || `custom_${Date.now()}`;
    addCustomProvider({
      providerKey: `custom_${key}`,
      displayName: name.trim(),
      baseUrl: baseUrl.trim(),
      logo: logoDataUrl ?? undefined,
      keys: [],
      models: [],
      active: true,
    });
    setName('');
    setBaseUrl('');
    setLogoDataUrl(null);
    setShowForm(false);
  };

  return (
    <div className="cp-root">
      <div className="r-card-t">
        <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
          <div className="dp-kpi-icon" style={{ width: 36, height: 36 }}>
            <Server size={18} />
          </div>
          <div>
            <h3 className="r-h r-h-3">Custom Providers</h3>
            <div className="cp-head-chips" style={{ marginTop: 4 }}>
              <span className="r-chip" style={{ fontSize: '10px' }}>
                {totalModels} models total
              </span>
              <span className="r-chip" style={{ fontSize: '10px' }}>
                {totalKeys} active keys
              </span>
            </div>
          </div>
        </div>
        <span className="r-chip" style={{ fontSize: '10px' }}>
          {activeProviders} / {totalProviders} online
        </span>
      </div>

      <div className="r-grid-4">
        {[
          { label: 'Providers', value: totalProviders, sub: `${activeProviders} active` },
          { label: 'Models', value: totalModels, sub: 'across providers' },
          { label: 'Keys', value: totalKeys, sub: 'active total' },
          { label: 'Templates', value: 5, sub: 'seeded' },
        ].map(s => (
          <div key={s.label} className="fp-stat">
            <div className="fp-stat-label">{s.label}</div>
            <div className="fp-stat-val">{s.value}</div>
            <div className="fp-stat-sub">{s.sub}</div>
          </div>
        ))}
      </div>

      {showForm && (
        <div className="cp-new-form">
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <Plus size={14} style={{ color: 'var(--color-ink)' }} />
            <span style={{ fontSize: '12px', fontWeight: 600, color: 'var(--color-ink)' }}>
              Add Custom Provider
            </span>
            <button
              onClick={() => { setShowForm(false); setName(''); setBaseUrl(''); setLogoDataUrl(null); }}
              className="r-btn r-btn-icon r-btn-ghost"
              style={{ marginLeft: 'auto' }}
            >
              <X size={12} />
            </button>
          </div>

          <div className="cp-new-grid">
            <div className="r-field">
              <input
                type="text"
                placeholder="Display name (e.g. Together AI)"
                value={name}
                onChange={e => setName(e.target.value)}
              />
            </div>
            <div className="r-field">
              <input
                type="text"
                placeholder="https://api.example.com/v1"
                value={baseUrl}
                onChange={e => setBaseUrl(e.target.value)}
                style={{ fontFamily: "'Geist Mono', ui-monospace, monospace" }}
              />
            </div>
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: 10, flexWrap: 'wrap' }}>
            <span style={{ fontSize: '10px', color: 'var(--color-muted)' }}>Logo (optional):</span>
            <input
              ref={fileRef}
              type="file"
              accept="image/*"
              style={{ display: 'none' }}
              onChange={e => handleFile(e.target.files?.[0])}
            />
            <button
              onClick={() => fileRef.current?.click()}
              className="r-btn"
              style={{ padding: '4px 10px' }}
            >
              <Edit3 size={10} />
              {logoDataUrl ? 'Change logo' : 'Pick image file'}
            </button>
            {logoDataUrl && (
              <>
                <div className="cp-logo-preview">
                  <img src={logoDataUrl} alt="Preview" />
                </div>
                <button
                  onClick={() => setLogoDataUrl(null)}
                  className="r-btn r-btn-icon r-btn-ghost"
                  title="Remove logo"
                >
                  <Trash2 size={11} />
                </button>
              </>
            )}
            <div style={{ marginLeft: 'auto' }}>
              <button
                onClick={handleSubmit}
                disabled={!name.trim() || !baseUrl.trim()}
                className="r-btn r-btn-primary"
              >
                <Check size={13} />
                Save Provider
              </button>
            </div>
          </div>
        </div>
      )}

      <div className="cp-list">
        {customProviders.map(p => (
          <ProviderCard key={p.id} provider={p} />
        ))}
      </div>

      {!showForm && (
        <button
          onClick={() => setShowForm(true)}
          className="cp-fab"
          title="Add Custom Provider"
        >
          <Plus size={22} strokeWidth={2.5} />
        </button>
      )}
    </div>
  );
}

export default CustomProvidersPanel;
