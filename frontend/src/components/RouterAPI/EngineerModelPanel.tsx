import { useEffect, useState } from 'react';
import { Cpu, Save, Check } from 'lucide-react';

const API = import.meta.env.VITE_API_URL || 'http://localhost:3111/api';

interface CuliModel {
  id:           string;
  display_name: string;
  description:  string;
  tier:         string;
}

interface EngineerModels {
  architect: string;
  coder:     string;
  reviewer:  string;
}

const ROLES = [
  { key: 'architect', label: '🏗 Architect', desc: 'Planning, system design, task decomposition' },
  { key: 'coder',     label: '💻 Coder',     desc: 'Implementation, code generation, refactoring' },
  { key: 'reviewer',  label: '🔍 Reviewer',  desc: 'Code review, security audit, testing' },
] as const;

const DEFAULTS: EngineerModels = {
  architect: 'culi-ultra',
  coder:     'culi-coder',
  reviewer:  'culi-pro',
};

export default function EngineerModelPanel() {
  const [models,   setModels]   = useState<CuliModel[]>([]);
  const [assigned, setAssigned] = useState<EngineerModels>(() => {
    try { return JSON.parse(localStorage.getItem('culi-engineer-models') || '{}'); }
    catch { return {}; }
  });
  const [saved,    setSaved]    = useState(false);
  const [loading,  setLoading]  = useState(true);

  useEffect(() => {
    fetch(`${API}/models`)
      .then(r => r.json())
      .then(d => setModels(d.models || []))
      .catch(() => {})
      .finally(() => setLoading(false));
  }, []);

  const get = (role: keyof EngineerModels) =>
    assigned[role] ?? DEFAULTS[role];

  const set = (role: keyof EngineerModels, model: string) =>
    setAssigned(prev => ({ ...prev, [role]: model }));

  const save = async () => {
    localStorage.setItem('culi-engineer-models', JSON.stringify(assigned));
    try {
      await fetch(`${API}/settings`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ engineer_models: assigned }),
      });
    } catch {}
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  const displayName = (id: string) =>
    models.find(m => m.id === id)?.display_name ?? id;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
        <Cpu size={18} />
        <div>
          <h3 style={{ margin: 0, fontSize: 14 }}>Engineer Model Assignment</h3>
          <p style={{ margin: 0, fontSize: 11, color: 'var(--color-muted)' }}>
            Assign CULI models to each agent role. Harness tasks always use Sixth AI (free, hidden).
          </p>
        </div>
      </div>

      {/* Harness notice */}
      <div style={{
        padding: '8px 12px', borderRadius: 8,
        background: 'var(--color-surface)', border: '1px solid var(--color-rule)',
        fontSize: 11, color: 'var(--color-muted)',
      }}>
        ⚡ <strong>Harness layer</strong> (Sixth AI + Blackbox) is used automatically for internal tasks —
        tool calls, memory summarization, and quick lookups. These run silently and never consume CULI model credits.
      </div>

      {/* Role assignment table */}
      <div style={{
        background: 'var(--color-paper-2)', border: '1px solid var(--color-rule)',
        borderRadius: 10, overflow: 'hidden',
      }}>
        {/* Header row */}
        <div style={{
          display: 'grid', gridTemplateColumns: '1fr 1fr 1fr',
          padding: '8px 14px', background: 'var(--color-surface)',
          borderBottom: '1px solid var(--color-rule)',
          fontSize: 10, fontWeight: 700, textTransform: 'uppercase',
          letterSpacing: '.05em', color: 'var(--color-muted)',
        }}>
          <span>Role</span><span>Model</span><span>Description</span>
        </div>

        {ROLES.map((role, i) => (
          <div key={role.key} style={{
            display: 'grid', gridTemplateColumns: '1fr 1fr 1fr',
            padding: '10px 14px', alignItems: 'center',
            borderBottom: i < ROLES.length - 1 ? '1px solid var(--color-rule)' : 'none',
          }}>
            <span style={{ fontSize: 12, fontWeight: 600 }}>{role.label}</span>
            <select
              value={get(role.key)}
              onChange={e => set(role.key, e.target.value)}
              disabled={loading}
              style={{
                padding: '4px 8px', fontSize: 12,
                background: 'var(--color-surface)', border: '1px solid var(--color-rule)',
                color: 'var(--color-ink)', borderRadius: 4,
                cursor: 'pointer',
              }}
            >
              {models.length > 0 ? models.filter(m => m.id !== 'culi-auto').map(m => (
                <option key={m.id} value={m.id}>{m.display_name}</option>
              )) : (
                <option value={get(role.key)}>{displayName(get(role.key))}</option>
              )}
            </select>
            <span style={{ fontSize: 10, color: 'var(--color-muted)' }}>{role.desc}</span>
          </div>
        ))}
      </div>

      {/* Save button */}
      <button
        onClick={save}
        style={{
          alignSelf: 'flex-start', display: 'inline-flex', alignItems: 'center', gap: 6,
          padding: '6px 16px', borderRadius: 6,
          background: saved ? 'var(--color-ink-2)' : 'var(--color-ink)',
          color: 'var(--color-paper-1)', fontSize: 12, fontWeight: 600, border: 0,
          cursor: 'pointer', transition: 'background .2s',
        }}
      >
        {saved ? <Check size={13} /> : <Save size={13} />}
        {saved ? 'Saved!' : 'Save Assignment'}
      </button>
    </div>
  );
}
