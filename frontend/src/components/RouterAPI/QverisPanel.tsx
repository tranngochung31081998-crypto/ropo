import { useState, useEffect } from 'react';
import {
  Key, Plus, Trash2, Edit3, Check, ChevronDown, ChevronRight,
  Save, RefreshCw, Server, Copy, Eye, EyeOff,
} from 'lucide-react';
import { useRouterStore, type ApiKeyEntry } from './store';
import { ProviderLogo } from './providerLogos';

const API = import.meta.env.VITE_API_URL || 'http://localhost:3111/api';
const MODEL_COUNT = 26;

function maskKey(key: string): string {
  if (!key) return 'N/A';
  if (key.length <= 8) return key.charAt(0) + '…' + key.charAt(key.length - 1);
  const prefix = key.startsWith('sk-') ? key.slice(0, 3) : key.slice(0, 2);
  const suffix = key.slice(-4);
  return `${prefix}…${suffix}`;
}

function formatCredits(credits: number | null | undefined): string {
  if (credits === null || credits === undefined) return 'N/A';
  if (credits >= 1000) return `$${(credits / 100).toFixed(2)}`;
  return `$${credits.toFixed(2)}`;
}

export default function QverisPanel() {
  const {
    qverisKeys,
    addQverisKey,
    removeQverisKey,
    toggleQverisKey,
    updateQverisCredits,
    qverisRotationPolicy: policy,
    setQverisRotationPolicy,
  } = useRouterStore();

  const [labelInput, setLabelInput] = useState('');
  const [keyInput, setKeyInput] = useState('');
  const [showKey, setShowKey] = useState<Record<string, boolean>>({});
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [editingCreditsId, setEditingCreditsId] = useState<string | null>(null);
  const [creditsDraft, setCreditsDraft] = useState('');

  const activeKeys = qverisKeys.filter(k => k.active);
  const totalCredits = activeKeys.reduce(
    (sum, k) => sum + (typeof k.credits === 'number' ? k.credits : 0),
    0
  );
  const totalRequests = qverisKeys.reduce(
    (sum, k) => sum + (k.requests || 0),
    0
  );
  const rotationActive = activeKeys.length >= 2;

  // Sync keys to backend when changed
  useEffect(() => {
    const timer = setTimeout(() => {
      fetch(`${API}/qveris/keys`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          keys: qverisKeys.map(k => ({ key: k.key, label: k.label, active: k.active }))
        }),
      }).catch(() => {});
    }, 500);
    return () => clearTimeout(timer);
  }, [qverisKeys]);

  const handleAdd = () => {
    if (!keyInput.trim()) return;
    addQverisKey(labelInput.trim(), keyInput.trim());
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

  const toggleVisibility = (id: string) => {
    setShowKey(prev => ({ ...prev, [id]: !prev[id] }));
  };

  const beginEditCredits = (k: ApiKeyEntry) => {
    setEditingCreditsId(k.id);
    setCreditsDraft(typeof k.credits === 'number' ? String(k.credits) : '');
  };

  const commitCredits = (id: string) => {
    const n = parseFloat(creditsDraft);
    updateQverisCredits(id, isNaN(n) ? null : n);
    setEditingCreditsId(null);
    setCreditsDraft('');
  };

  const maxCredits = Math.max(totalCredits, 100);
  const creditsPct = totalCredits > 0 ? Math.min(100, (totalCredits / maxCredits) * 100) : 0;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
      {/* ── Header Row ────────────────────────────────────────── */}
      <div style={{
        display: 'flex', alignItems: 'center', justifyContent: 'space-between',
        flexWrap: 'wrap', gap: '0.5rem',
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '0.6rem' }}>
          <ProviderLogo provider="qveris" displayName="Qveris" size={30} />
          <div>
            <h3 style={{ fontSize: '15px', margin: 0, lineHeight: 1.2 }}>
              Qveris Key Manager
            </h3>
            <div style={{
              display: 'flex', alignItems: 'center', gap: '0.5rem',
              marginTop: '2px',
            }}>
              <span style={{ fontSize: '10px', color: 'var(--color-muted)' }}>
                {MODEL_COUNT} models supported
              </span>
              <span style={{
                fontSize: '9px', color: 'var(--color-muted)',
                fontFamily: "'Geist Mono', monospace",
              }}>
                · v1.0
              </span>
            </div>
          </div>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
          {rotationActive && (
            <div style={{
              display: 'inline-flex', alignItems: 'center', gap: '6px',
              padding: '4px 10px',
              background: 'var(--color-surface)',
              border: '1px solid var(--color-rule)',
              color: 'var(--color-ink)',
              fontSize: '10px', fontWeight: 600,
              borderRadius: '999px',
              letterSpacing: '0.02em',
            }}>
              <RefreshCw size={11} style={{ animation: 'sp 1.2s linear infinite' }} />
              Key rotation active
            </div>
          )}
          <div style={{
            padding: '4px 10px',
            background: 'var(--color-surface)',
            border: '1px solid var(--color-rule)',
            fontSize: '10px', fontWeight: 600,
            color: 'var(--color-ink-2)',
            borderRadius: '6px',
          }}>
            {qverisKeys.length} / {activeKeys.length} active
          </div>
        </div>
      </div>

      {/* ── Summary Bar ───────────────────────────────────────── */}
      <div style={{
        display: 'grid',
        gridTemplateColumns: 'repeat(3, 1fr)',
        gap: '0.6rem',
      }}>
        <div style={{
          padding: '0.7rem 0.8rem',
          background: 'var(--color-paper-2)',
          border: '1px solid var(--color-rule)',
          borderRadius: '10px',
        }}>
          <div style={{ fontSize: '9px', color: 'var(--color-muted)', textTransform: 'uppercase', letterSpacing: '0.05em', fontWeight: 600 }}>
            Total Keys
          </div>
          <div style={{
            fontFamily: 'var(--font-body)',
            fontSize: '22px', fontWeight: 700,
            color: 'var(--color-ink)', lineHeight: 1.1, marginTop: '4px',
          }}>
            {qverisKeys.length}
          </div>
          <div style={{ fontSize: '10px', color: 'var(--color-muted)', marginTop: '2px' }}>
            {activeKeys.length} online · {qverisKeys.length - activeKeys.length} idle
          </div>
        </div>

        <div style={{
          padding: '0.7rem 0.8rem',
          background: 'var(--color-paper-2)',
          border: '1px solid var(--color-rule)',
          borderRadius: '10px',
        }}>
          <div style={{ fontSize: '9px', color: 'var(--color-muted)', textTransform: 'uppercase', letterSpacing: '0.05em', fontWeight: 600 }}>
            Total Credits
          </div>
          <div style={{
            fontFamily: 'var(--font-body)',
            fontSize: '22px', fontWeight: 700,
            color: totalCredits > 0 ? 'var(--color-ink)' : 'var(--color-ink)',
            lineHeight: 1.1, marginTop: '4px',
          }}>
            {totalCredits > 0 ? `$${(totalCredits / 100).toFixed(2)}` : '$0.00'}
          </div>
          <div style={{ marginTop: '8px' }}>
            <div style={{
              width: '100%', height: '4px',
              background: 'var(--color-paper-1)',
              borderRadius: '2px', overflow: 'hidden',
            }}>
              <div style={{
                width: `${creditsPct}%`, height: '100%',
                background: 'var(--color-ink)',
                transition: 'width 0.35s ease',
              }} />
            </div>
            <div style={{
              display: 'flex', justifyContent: 'space-between',
              marginTop: '3px',
              fontSize: '8px', color: 'var(--color-muted)',
              fontFamily: "'Geist Mono', monospace",
            }}>
              <span>{activeKeys.filter(k => typeof k.credits === 'number').length} with balance</span>
              <span>of ${(maxCredits / 100).toFixed(2)}</span>
            </div>
          </div>
        </div>

        <div style={{
          padding: '0.7rem 0.8rem',
          background: 'var(--color-paper-2)',
          border: '1px solid var(--color-rule)',
          borderRadius: '10px',
        }}>
          <div style={{ fontSize: '9px', color: 'var(--color-muted)', textTransform: 'uppercase', letterSpacing: '0.05em', fontWeight: 600 }}>
            Requests Served
          </div>
          <div style={{
            fontFamily: 'var(--font-body)',
            fontSize: '22px', fontWeight: 700,
            color: 'var(--color-ink)', lineHeight: 1.1, marginTop: '4px',
          }}>
            {totalRequests.toLocaleString()}
          </div>
          <div style={{
            display: 'flex', alignItems: 'center', gap: '4px',
            fontSize: '10px', color: 'var(--color-muted)', marginTop: '2px',
          }}>
            <Key size={10} />
            {MODEL_COUNT} models available
          </div>
        </div>
      </div>

      {/* ── Add Key Inline Form ───────────────────────────────── */}
      <div style={{
        padding: '0.75rem',
        background: 'var(--color-paper-2)',
        border: '1px solid var(--color-rule)',
        borderRadius: '10px',
      }}>
        <div style={{
          display: 'flex', alignItems: 'center', gap: '0.5rem', marginBottom: '0.5rem',
        }}>
          <Plus size={13} style={{ color: 'var(--color-ink)' }} />
          <span style={{ fontSize: '11px', fontWeight: 600, color: 'var(--color-ink-2)' }}>
            Add Qveris API Key
          </span>
        </div>
        <div style={{
          display: 'grid',
          gridTemplateColumns: '1fr 1.5fr auto',
          gap: '0.4rem',
        }}>
          <input
            type="text"
            placeholder="Label (e.g. Production-1)"
            value={labelInput}
            onChange={e => setLabelInput(e.target.value)}
            style={{
              padding: '0.45rem 0.6rem',
              background: 'var(--color-surface)',
              border: '1px solid var(--color-rule)',
              fontSize: '12px', color: 'var(--color-ink)',
              outline: 'none',
            }}
            onKeyDown={e => { if (e.key === 'Enter') handleAdd(); }}
          />
          <input
            type="password"
            placeholder="qv-xxxxxxxxxxxxxxxxxxxxxxxx"
            value={keyInput}
            onChange={e => setKeyInput(e.target.value)}
            style={{
              padding: '0.45rem 0.6rem',
              background: 'var(--color-surface)',
              border: '1px solid var(--color-rule)',
              fontSize: '12px', color: 'var(--color-ink)',
              outline: 'none',
              fontFamily: "'Geist Mono', monospace",
            }}
            onKeyDown={e => { if (e.key === 'Enter') handleAdd(); }}
          />
          <button
            onClick={handleAdd}
            disabled={!keyInput.trim()}
            style={{
              padding: '0 1rem',
              background: 'var(--color-ink)',
              color: 'var(--color-paper-1)',
              fontSize: '11px', fontWeight: 600,
              border: 0,
              cursor: keyInput.trim() ? 'pointer' : 'not-allowed',
              opacity: keyInput.trim() ? 1 : 0.4,
              display: 'inline-flex', alignItems: 'center', gap: '6px',
              whiteSpace: 'nowrap',
            }}
          >
            <Plus size={13} />
            Add Key
          </button>
        </div>
      </div>

      {/* ── Keys Table ────────────────────────────────────────── */}
      <div style={{
        background: 'var(--color-paper-2)',
        border: '1px solid var(--color-rule)',
        borderRadius: '10px',
        overflow: 'hidden',
      }}>
        <div style={{
          display: 'grid',
          gridTemplateColumns: '1.2fr 1.8fr 1fr 0.9fr 0.8fr 0.7fr',
          gap: '0.5rem',
          padding: '0.55rem 0.7rem',
          background: 'var(--color-surface)',
          borderBottom: '1px solid var(--color-rule)',
        }}>
          {['Label', 'Key', 'Credits', 'Requests', 'Active', ''].map(h => (
            <div
              key={h}
              style={{
                fontSize: '9px', fontWeight: 700,
                textTransform: 'uppercase', letterSpacing: '0.05em',
                color: 'var(--color-muted)',
              }}
            >
              {h}
            </div>
          ))}
        </div>

        {qverisKeys.length === 0 ? (
          <div style={{
            padding: '2rem 1rem',
            textAlign: 'center',
            color: 'var(--color-muted)',
          }}>
            <Key size={28} style={{ opacity: 0.4, marginBottom: '0.5rem' }} />
            <div style={{ fontSize: '12px', fontWeight: 500, color: 'var(--color-ink-2)' }}>
              No Qveris keys yet
            </div>
            <div style={{ fontSize: '10px', marginTop: '3px' }}>
              Add your first key above to enable Qveris routing
            </div>
          </div>
        ) : (
          qverisKeys.map((k, idx) => (
            <div
              key={k.id}
              style={{
                display: 'grid',
                gridTemplateColumns: '1.2fr 1.8fr 1fr 0.9fr 0.8fr 0.7fr',
                gap: '0.5rem',
                padding: '0.55rem 0.7rem',
                alignItems: 'center',
                borderBottom: idx < qverisKeys.length - 1 ? '1px solid var(--color-rule)' : 'none',
                background: k.active ? 'transparent' : 'var(--color-surface)',
                opacity: k.active ? 1 : 0.6,
              }}
            >
              {/* Label */}
              <div style={{
                display: 'flex', alignItems: 'center', gap: '6px',
                minWidth: 0,
              }}>
                <ProviderLogo provider="qveris" size={16} />
                <span style={{
                  fontSize: '12px', fontWeight: 500,
                  color: 'var(--color-ink)',
                  whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis',
                }}>
                  {k.label}
                </span>
              </div>

              {/* Key (masked + actions) */}
              <div style={{
                display: 'flex', alignItems: 'center', gap: '4px',
                minWidth: 0,
              }}>
                <code style={{
                  fontSize: '11px',
                  color: 'var(--color-ink-2)',
                  background: 'var(--color-paper-1)',
                  padding: '2px 6px',
                  border: '1px solid var(--color-rule)',
                  fontFamily: "'Geist Mono', monospace",
                  whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis',
                  flex: 1, minWidth: 0,
                }}>
                  {showKey[k.id] ? k.key : maskKey(k.key)}
                </code>
                <button
                  className="icon-btn"
                  style={{ width: 22, height: 22 }}
                  onClick={() => toggleVisibility(k.id)}
                  title={showKey[k.id] ? 'Hide' : 'Reveal'}
                >
                  {showKey[k.id] ? <EyeOff size={12} /> : <Eye size={12} />}
                </button>
                <button
                  className="icon-btn"
                  style={{ width: 22, height: 22 }}
                  onClick={() => handleCopy(k.key, k.id)}
                  title="Copy"
                >
                  {copiedId === k.id ? (
                    <Check size={12} style={{ color: 'var(--color-ink)' }} />
                  ) : (
                    <Copy size={12} />
                  )}
                </button>
              </div>

              {/* Credits */}
              <div>
                {editingCreditsId === k.id ? (
                  <div style={{ display: 'flex', gap: '3px', alignItems: 'center' }}>
                    <input
                      type="number"
                      step="0.01"
                      value={creditsDraft}
                      onChange={e => setCreditsDraft(e.target.value)}
                      onKeyDown={e => { if (e.key === 'Enter') commitCredits(k.id); }}
                      onBlur={() => commitCredits(k.id)}
                      autoFocus
                      style={{
                        width: '80px',
                        padding: '2px 6px',
                        fontSize: '11px',
                        background: 'var(--color-surface)',
                        border: '1px solid var(--color-ink)',
                        color: 'var(--color-ink)',
                        outline: 'none',
                      }}
                    />
                    <button
                      className="icon-btn"
                      style={{ width: 20, height: 20, color: 'var(--color-ink)' }}
                      onClick={() => commitCredits(k.id)}
                    >
                      <Check size={11} />
                    </button>
                  </div>
                ) : (
                  <button
                    onClick={() => beginEditCredits(k)}
                    style={{
                      display: 'inline-flex', alignItems: 'center', gap: '4px',
                      fontSize: '11px',
                      color: typeof k.credits === 'number' ? 'var(--color-ink-2)' : 'var(--color-muted)',
                      fontFamily: typeof k.credits === 'number' ? "'Geist Mono', monospace" : undefined,
                      cursor: 'pointer',
                      background: 'transparent',
                      border: 0,
                      padding: 0,
                    }}
                    title="Click to edit credits"
                  >
                    <span>{formatCredits(k.credits)}</span>
                    <Edit3 size={10} style={{ opacity: 0.5, flexShrink: 0 }} />
                  </button>
                )}
              </div>

              {/* Requests */}
              <div style={{
                fontSize: '11px',
                color: 'var(--color-ink-2)',
                fontFamily: "'Geist Mono', monospace",
              }}>
                {(k.requests || 0).toLocaleString()}
              </div>

              {/* Active toggle */}
              <div>
                <label className="tgl-sm">
                  <input
                    type="checkbox"
                    checked={k.active}
                    onChange={() => toggleQverisKey(k.id)}
                  />
                  <span className="tgl-sm-track" />
                </label>
              </div>

              {/* Actions */}
              <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
                <button
                  className="icon-btn"
                  style={{ width: 26, height: 26, color: 'var(--color-muted)' }}
                  onClick={() => removeQverisKey(k.id)}
                  title="Delete key"
                >
                  <Trash2 size={13} />
                </button>
              </div>
            </div>
          ))
        )}
      </div>

      {/* Rotation Policy Section */}
      <div style={{
        background: 'var(--color-paper-2)', border: '1px solid var(--color-rule)',
        borderRadius: 10, padding: '0.75rem',
      }}>
        <div style={{ fontSize: 11, fontWeight: 700, color: 'var(--color-ink-2)', marginBottom: 10, textTransform: 'uppercase', letterSpacing: '.05em' }}>
          <RefreshCw size={12} style={{ display: 'inline', marginRight: 5 }} />
          Key Rotation Policy
        </div>

        {[
          { field: 'onRateLimit',    label: 'Rotate on rate limit (429)',    desc: 'Switch key when throttled' },
          { field: 'onOutOfCredits', label: 'Rotate on out of credits (402)', desc: 'Switch key when balance depleted' },
          { field: 'onAuthError',    label: 'Rotate on auth error (401)',    desc: 'Remove and rotate dead keys' },
          { field: 'roundRobin',     label: 'Round-robin rotation',          desc: 'Spread load across all keys evenly' },
        ].map(opt => (
          <div key={opt.field} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 8 }}>
            <div>
              <div style={{ fontSize: 12, color: 'var(--color-ink)' }}>{opt.label}</div>
              <div style={{ fontSize: 10, color: 'var(--color-muted)' }}>{opt.desc}</div>
            </div>
            <label className="tgl">
              <input
                type="checkbox"
                checked={!!policy[opt.field as keyof typeof policy]}
                onChange={() => setQverisRotationPolicy({ [opt.field]: !policy[opt.field as keyof typeof policy] })}
              />
              <span className="tgl-track" />
            </label>
          </div>
        ))}

        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginTop: 8 }}>
          <span style={{ fontSize: 11, color: 'var(--color-ink)' }}>Min credits threshold</span>
          <span style={{ fontSize: 10, color: 'var(--color-muted)' }}>(rotate before balance drops below)</span>
          <input
            type="number" step="0.01" min="0"
            value={policy.minCreditsUsd}
            onChange={e => setQverisRotationPolicy({ minCreditsUsd: parseFloat(e.target.value) || 0 })}
            style={{ width: 70, padding: '2px 6px', fontSize: 11, background: 'var(--color-surface)', border: '1px solid var(--color-rule)', color: 'var(--color-ink)' }}
          />
          <span style={{ fontSize: 10 }}>USD</span>
        </div>
      </div>
    </div>
  );
}
