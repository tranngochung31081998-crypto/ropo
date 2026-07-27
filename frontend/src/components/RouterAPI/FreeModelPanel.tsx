import { useEffect, useRef, useState } from 'react';
import { CheckCircle2, ArrowRight, Database, Cpu, Shield, Zap, RefreshCw, WifiOff } from 'lucide-react';
import { ProviderLogo } from './providerLogos';
import { getRouterStats, getRouterModels, type RouterStats, type RouterModel } from '../../api/routerClient';

// Failover strategy — static (reflects real router logic)
const tierModels = [
  { tier: 'Auto', tag: 'Router pick', desc: 'Best-fit by cost/latency signal' },
  { tier: 'Fast Tier', tag: 'Latency-first', desc: 'deepseek-v4-flash via Blackbox' },
  { tier: 'Premium Tier', tag: 'Quality-first', desc: 'claude-fable-5 via Sixth AI' },
];

type ChipStatus = 'ok' | 'warn' | 'err';

interface LiveModel {
  id: string;
  label: string;
  sub: string;
  provider: 'blackbox' | 'sixth';
  status: ChipStatus;
}

function statusLabel(s: ChipStatus) {
  return s === 'ok' ? 'LIVE' : s === 'warn' ? 'DEGRADED' : 'DOWN';
}

export default function FreeModelPanel() {
  const [stats, setStats]     = useState<RouterStats | null>(null);
  const [models, setModels]   = useState<LiveModel[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError]     = useState<string | null>(null);
  const intervalRef           = useRef<ReturnType<typeof setInterval> | null>(null);

  async function fetchData() {
    try {
      const [s, m] = await Promise.all([getRouterStats(), getRouterModels()]);
      setStats(s);

      // Build live model chips from real /v1/models response
      const freeModels: LiveModel[] = (m.data ?? [])
        .filter(mdl => mdl.provider === 'blackbox' || mdl.provider === 'sixth')
        .map(mdl => {
          const provider = mdl.provider as 'blackbox' | 'sixth';
          const healthy  = provider === 'blackbox'
            ? s.providers.blackbox.healthy
            : s.providers.sixth.healthy;
          return {
            id:       mdl.id,
            label:    mdl.id,
            sub:      mdl.display_name,
            provider,
            status:   healthy ? 'ok' : 'err',
          };
        });

      setModels(freeModels.length > 0 ? freeModels : fallbackModels(s));
      setError(null);
    } catch (e) {
      setError('Router offline');
      setModels(fallbackModels(null));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    fetchData();
    intervalRef.current = setInterval(fetchData, 8000);
    return () => { if (intervalRef.current) clearInterval(intervalRef.current); };
  }, []);

  // ── Derived stats ──────────────────────────────────────────────────────
  const bb         = stats?.providers.blackbox;
  const sixth       = stats?.providers.sixth;
  const freeReqs    = (bb?.totalRequests ?? 0) + (sixth?.totalRequests ?? 0);
  const freeErrors  = (bb?.totalErrors ?? 0)   + (sixth?.totalErrors ?? 0);
  const successRate = freeReqs > 0 ? (freeReqs - freeErrors) / freeReqs : null;
  const freeHealthy = bb?.healthy || sixth?.healthy;
  const poolSize    = sixth?.poolSize ?? 0;
  const activeAcc   = sixth?.activeAccounts ?? 0;

  return (
    <div className="fp">
      <article className="fp-card">

        {/* ── Header ─────────────────────────────────────────────────── */}
        <header className="fp-h">
          <div className="fp-h-l">
            <span className="fp-ic" style={{ display: 'inline-flex', gap: 6 }}>
              <ProviderLogo provider="blackbox" displayName="Blackbox" size={26} />
              <ProviderLogo provider="sixth"    displayName="Sixth AI"  size={26} />
            </span>
            <div className="fp-h-t">
              <div className="fp-h-r1">
                <h2 className="fp-name">CULI FreeModel</h2>
                <span className={`fp-badge ${freeHealthy ? 'fp-badge-ok' : 'fp-badge-warn'}`}>
                  <span className="fp-badge-d" />
                  {loading ? 'loading…' : freeHealthy ? 'operational' : 'degraded'}
                </span>
                {error && (
                  <span className="fp-badge fp-badge-warn">
                    <WifiOff size={10} /> {error}
                  </span>
                )}
              </div>
              <p className="fp-sub">
                Unified cluster · Blackbox + Sixth routed internally · no keys required
                {sixth?.currentAccount && (
                  <> · active: <code>{sixth.currentAccount}</code></>
                )}
              </p>
            </div>
          </div>

          <div className="fp-h-r">
            <div className="fp-pill">
              <Cpu size={12} />
              <span>{loading ? '…' : `${models.length} models`}</span>
            </div>
            <div className="fp-pill fp-pill-accent">
              <Shield size={12} />
              <span>pool: {poolSize} · active: {activeAcc}</span>
            </div>
            <button className="fp-refresh-btn" onClick={fetchData} title="Refresh">
              <RefreshCw size={12} className={loading ? 'fp-spin' : ''} />
            </button>
          </div>
        </header>

        {/* ── Live Model Chips ────────────────────────────────────────── */}
        <section className="fp-sec">
          <div className="fp-sec-h">
            <span className="fp-sec-l"><Zap size={12} /> Available Models</span>
            <span className="fp-sec-s">{loading ? 'Loading…' : `${models.length} from /v1/models`}</span>
          </div>

          <div className="fp-chips" style={{ gridTemplateColumns: `repeat(${Math.min(models.length || 3, 4)}, 1fr)` }}>
            {(loading ? placeholderChips : models).map((c) => (
              <button key={c.id} type="button" className={`fp-chip fp-chip-${c.status}`}>
                <ProviderLogo provider={c.provider} size={18} />
                <span className="fp-chip-body">
                  <strong className="fp-chip-name">{c.label}</strong>
                  <span className="fp-chip-sub">{c.sub}</span>
                </span>
                <span className={`fp-chip-tag fp-tag-${c.status}`}>
                  {statusLabel(c.status)}
                </span>
              </button>
            ))}
          </div>
        </section>

        {/* ── Failover Strategy ───────────────────────────────────────── */}
        <section className="fp-sec">
          <div className="fp-sec-h">
            <span className="fp-sec-l"><ArrowRight size={12} /> Failover Strategy</span>
            <span className="fp-sec-s">Cascade on 5xx / timeout</span>
          </div>
          <ol className="fp-steps">
            {tierModels.map((t, i) => {
              const done   = i < 2;
              const active = i === 0;
              return (
                <li key={t.tier} className={`fp-step ${done ? 'fp-step-done' : ''} ${active ? 'fp-step-active' : ''}`}>
                  <div className="fp-step-row">
                    <span className={`fp-step-marker ${done ? 'fp-marker-done' : ''} ${active ? 'fp-marker-active' : ''}`}>
                      {done ? <CheckCircle2 size={12} /> : i + 1}
                    </span>
                    <div className="fp-step-copy">
                      <div className="fp-step-r1">
                        <strong className="fp-step-name">{t.tier}</strong>
                        <span className="fp-step-tag">{t.tag}</span>
                      </div>
                      <span className="fp-step-desc">{t.desc}</span>
                    </div>
                    {i < tierModels.length - 1 && <span className="fp-step-arr"><ArrowRight size={12} /></span>}
                  </div>
                  {i < tierModels.length - 1 && (
                    <div className={`fp-step-track ${done ? 'fp-track-done' : ''}`} />
                  )}
                </li>
              );
            })}
          </ol>
        </section>

        {/* ── Usage Stats ─────────────────────────────────────────────── */}
        <section className="fp-sec fp-sec-last">
          <div className="fp-sec-h">
            <span className="fp-sec-l"><Database size={12} /> Usage Stats</span>
            <span className="fp-sec-s">{loading ? 'loading…' : 'live from /stats'}</span>
          </div>

          <div className="fp-use">
            {/* Requests */}
            <div className="fp-use-card">
              <span className="fp-use-lbl">Requests</span>
              <strong className="fp-use-v">{loading ? '…' : freeReqs.toLocaleString('en-US')}</strong>
              <div className="fp-use-bar">
                <span className="fp-use-bar-f" style={{ width: `${Math.min((freeReqs / 10000) * 100, 100)}%` }} />
              </div>
              <span className="fp-use-cap">
                {bb?.totalRequests ?? 0} blackbox · {sixth?.totalRequests ?? 0} sixth
              </span>
            </div>

            {/* Success Rate */}
            <div className="fp-use-card">
              <span className="fp-use-lbl">Success Rate</span>
              <strong className={`fp-use-v ${successRate !== null && successRate >= 0.95 ? 'fp-ok' : ''}`}>
                {loading ? '…' : successRate !== null ? `${(successRate * 100).toFixed(1)}%` : 'N/A'}
              </strong>
              <div className={`fp-use-bar ${successRate !== null && successRate >= 0.95 ? 'fp-use-bar-ok' : ''}`}>
                <span className="fp-use-bar-f" style={{ width: successRate !== null ? `${(successRate * 100).toFixed(1)}%` : '0%' }} />
              </div>
              <span className="fp-use-cap">
                {freeErrors > 0 && <><CheckCircle2 size={10} /> {freeErrors} failed · </>}
                {freeReqs - freeErrors} succeeded
              </span>
            </div>

            {/* Pool */}
            <div className="fp-use-card">
              <span className="fp-use-lbl">Sixth Pool</span>
              <strong className="fp-use-v">
                {loading ? '…' : `${activeAcc}`}
                <span className="fp-unit"> / {poolSize}</span>
              </strong>
              <div className="fp-use-bar fp-use-bar-warn">
                <span className="fp-use-bar-f" style={{ width: poolSize > 0 ? `${(activeAcc / poolSize) * 100}%` : '0%' }} />
              </div>
              <span className="fp-use-cap">active accounts in pool</span>
            </div>

            {/* Cost */}
            <div className="fp-use-card">
              <span className="fp-use-lbl">Tokens / Cost</span>
              <strong className="fp-use-v">—<span className="fp-unit"> · $0.00</span></strong>
              <div className="fp-use-bar">
                <span className="fp-use-bar-f" style={{ width: '0%' }} />
              </div>
              <span className="fp-use-cap">Free tier · $0 billed</span>
            </div>
          </div>
        </section>
      </article>

      <style>{`
        .fp { padding: var(--space-md) 0; }
        .fp-card {
          background: var(--color-paper-2); border: 1px solid var(--color-rule);
          border-radius: var(--radius-lg); padding: var(--space-lg);
          display: flex; flex-direction: column; gap: var(--space-lg);
        }
        .fp-h { display: flex; align-items: flex-start; justify-content: space-between; gap: var(--space-md); flex-wrap: wrap; }
        .fp-h-l { display: flex; align-items: flex-start; gap: var(--space-sm); }
        .fp-ic { width: 44px; height: 44px; display: grid; place-items: center; background: var(--color-surface); border: 1px solid var(--color-rule); border-radius: var(--radius-lg); flex-shrink: 0; }
        .fp-h-t { display: flex; flex-direction: column; gap: 2px; }
        .fp-h-r1 { display: flex; align-items: center; gap: var(--space-sm); flex-wrap: wrap; }
        .fp-name { font-size: var(--text-lg); font-weight: 700; color: var(--color-ink); }
        .fp-sub  { font-size: var(--text-sm); color: var(--color-muted); max-width: 54ch; }
        .fp-badge { display: inline-flex; align-items: center; gap: 6px; padding: 3px 9px; border-radius: var(--radius-full); font-size: var(--text-xs); font-weight: 600; text-transform: uppercase; letter-spacing: .05em; background: var(--color-surface); border: 1px solid var(--color-rule); }
        .fp-badge-ok   { color: var(--color-ink-2); }
        .fp-badge-warn { color: var(--color-muted); }
        .fp-badge-d { width: 6px; height: 6px; border-radius: 50%; background: currentColor; box-shadow: 0 0 6px currentColor; }
        .fp-h-r { display: flex; align-items: center; gap: var(--space-xs); flex-wrap: wrap; }
        .fp-pill { display: inline-flex; align-items: center; gap: 5px; padding: 4px 10px; border-radius: var(--radius-full); border: 1px solid var(--color-rule); background: var(--color-surface); font-size: var(--text-xs); color: var(--color-ink-2); }
        .fp-pill-accent { color: var(--color-ink-2); }
        .fp-refresh-btn { display: grid; place-items: center; padding: 5px; border-radius: var(--radius-sm); background: var(--color-surface); border: 1px solid var(--color-rule); color: var(--color-ink-2); cursor: pointer; }
        .fp-refresh-btn:hover { background: var(--color-paper-2); }
        @keyframes spin { to { transform: rotate(360deg); } }
        .fp-spin { animation: spin .8s linear infinite; }

        .fp-sec { display: flex; flex-direction: column; gap: var(--space-sm); }
        .fp-sec:not(:last-child) { padding-bottom: var(--space-md); border-bottom: 1px solid var(--color-rule); }
        .fp-sec-last { padding-bottom: 0; border-bottom: 0 !important; }
        .fp-sec-h { display: flex; align-items: baseline; justify-content: space-between; }
        .fp-sec-l { display: inline-flex; align-items: center; gap: 6px; font-size: var(--text-xs); font-weight: 700; text-transform: uppercase; letter-spacing: .06em; color: var(--color-ink-2); }
        .fp-sec-s { font-size: var(--text-xs); color: var(--color-muted); font-family: 'Geist Mono', monospace; }

        .fp-chips { display: grid; gap: var(--space-sm); }
        @media (max-width: 900px) { .fp-chips { grid-template-columns: 1fr !important; } }
        .fp-chip { display: flex; align-items: center; gap: var(--space-xs); padding: var(--space-sm); background: var(--color-surface); border: 1px solid var(--color-rule); border-radius: var(--radius-md); text-align: left; transition: border-color .18s, background .18s, transform .18s; }
        .fp-chip:hover { border-color: var(--color-ink); background: color-mix(in srgb, var(--color-surface) 60%, var(--color-paper-2)); }
        .fp-chip-body { display: flex; flex-direction: column; gap: 2px; flex: 1; min-width: 0; }
        .fp-chip-name { font-size: var(--text-sm); font-weight: 600; color: var(--color-ink); font-family: 'Geist Mono', monospace; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
        .fp-chip-sub  { font-size: var(--text-xs); color: var(--color-muted); }
        .fp-chip-tag  { font-size: var(--text-xs); padding: 1px 7px; border-radius: var(--radius-sm); font-weight: 600; }
        .fp-tag-ok    { background: var(--color-surface); border: 1px solid var(--color-rule); color: var(--color-ink-2); }
        .fp-tag-warn  { background: var(--color-surface); border: 1px solid var(--color-rule); color: var(--color-muted); }
        .fp-tag-err   { background: var(--color-surface); border: 1px solid var(--color-rule); color: var(--color-muted); opacity: .6; }

        .fp-steps { list-style: none; display: flex; flex-direction: column; }
        .fp-step { position: relative; }
        .fp-step-row { display: grid; grid-template-columns: 32px minmax(0,1fr) auto; align-items: center; gap: var(--space-xs); padding: var(--space-sm) 0; position: relative; z-index: 1; }
        .fp-step-marker { width: 28px; height: 28px; display: grid; place-items: center; border: 1px solid var(--color-rule); background: var(--color-surface); border-radius: var(--radius-md); font-size: var(--text-xs); font-weight: 700; color: var(--color-muted); }
        .fp-marker-done   { border-color: var(--color-ink-2); color: var(--color-ink-2); background: var(--color-paper-2); }
        .fp-marker-active { border-color: var(--color-ink); background: var(--color-ink); color: var(--color-paper-1); box-shadow: 0 0 0 3px var(--color-surface); }
        .fp-step-copy { display: flex; flex-direction: column; gap: 2px; }
        .fp-step-r1   { display: flex; align-items: center; gap: var(--space-xs); flex-wrap: wrap; }
        .fp-step-name { font-size: var(--text-sm); font-weight: 600; color: var(--color-ink); }
        .fp-step-tag  { font-size: var(--text-xs); padding: 1px 7px; border-radius: var(--radius-sm); background: var(--color-surface); color: var(--color-muted); border: 1px solid var(--color-rule); }
        .fp-step-desc { font-size: var(--text-xs); color: var(--color-muted); font-family: 'Geist Mono', monospace; }
        .fp-step-arr  { color: var(--color-muted); }
        .fp-step-track { position: absolute; left: 14px; top: calc(var(--space-sm) + 28px); width: 2px; height: calc(100% - var(--space-sm) - 28px); background: var(--color-rule); }
        .fp-track-done { background: var(--color-ink-2); }

        .fp-use { display: grid; grid-template-columns: repeat(4,1fr); gap: var(--space-sm); }
        @media (max-width:1100px) { .fp-use { grid-template-columns: repeat(2,1fr); } }
        @media (max-width:600px)  { .fp-use { grid-template-columns: 1fr; } }
        .fp-use-card { background: var(--color-surface); border: 1px solid var(--color-rule); border-radius: var(--radius-md); padding: var(--space-sm); display: flex; flex-direction: column; gap: var(--space-xs); }
        .fp-use-lbl  { font-size: var(--text-xs); color: var(--color-muted); font-weight: 600; text-transform: uppercase; letter-spacing: .05em; }
        .fp-use-v    { font-size: var(--text-md); font-weight: 700; color: var(--color-ink); font-variant-numeric: tabular-nums; }
        .fp-use-v.fp-ok { color: var(--color-ink-2); }
        .fp-unit     { font-size: var(--text-sm); font-weight: 500; color: var(--color-muted); font-family: 'Geist Mono', monospace; }
        .fp-use-bar  { height: 5px; background: var(--color-paper-2); border-radius: 3px; overflow: hidden; }
        .fp-use-bar-f { display: block; height: 100%; background: var(--color-ink); border-radius: 3px; transition: width .35s ease; }
        .fp-use-bar-ok   .fp-use-bar-f { background: var(--color-ink); }
        .fp-use-bar-warn .fp-use-bar-f { background: var(--color-ink); }
        .fp-use-cap { display: inline-flex; align-items: center; gap: 4px; font-size: var(--text-xs); color: var(--color-muted); font-family: 'Geist Mono', monospace; }
      `}</style>
    </div>
  );
}

// ── helpers ──────────────────────────────────────────────────────────────
function fallbackModels(s: RouterStats | null): LiveModel[] {
  return [
    { id: 'deepseek-v4-flash', label: 'deepseek-v4-flash', sub: 'Fast Tier', provider: 'blackbox', status: s?.providers.blackbox.healthy ? 'ok' : 'err' },
    { id: 'claude-fable-5',    label: 'claude-fable-5',    sub: 'Premium Tier', provider: 'sixth', status: s?.providers.sixth.healthy ? 'ok' : 'err' },
    { id: 'gpt-4.1-mini',      label: 'gpt-4.1-mini',      sub: 'Premium Tier', provider: 'sixth', status: s?.providers.sixth.healthy ? 'ok' : 'warn' },
  ];
}

const placeholderChips: LiveModel[] = [
  { id: 'loading-1', label: 'Loading…', sub: '', provider: 'blackbox', status: 'warn' },
  { id: 'loading-2', label: 'Loading…', sub: '', provider: 'sixth',    status: 'warn' },
  { id: 'loading-3', label: 'Loading…', sub: '', provider: 'sixth',    status: 'warn' },
];
