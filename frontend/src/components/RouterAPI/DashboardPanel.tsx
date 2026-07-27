import { useEffect, useRef, useState } from 'react';
import { useRouterStore } from './store';
import {
  Server, Zap, Shield, RefreshCw, Activity, CheckCircle2,
  AlertCircle, Clock, Database, Cpu, Wifi, WifiOff,
} from 'lucide-react';
import { ProviderLogo } from './providerLogos';
import { getRouterStats, getRouterHealth, type RouterStats, type RouterHealth } from '../../api/routerClient';
import { getHealth as getCuliHealth } from '../../api/client';

function fmtInt(n: number) { return n.toLocaleString('en-US'); }
function fmtPct(v: number)  { return `${(v * 100).toFixed(1)}%`; }
function fmtUptime(seconds: number) {
  if (seconds < 60)   return `${seconds}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h`;
  return `${Math.floor(seconds / 86400)}d`;
}

// ─── Health bar colour helpers ────────────────────────────────────────────
function providerStatus(stat?: { healthy: boolean }) {
  if (!stat) return 'err';
  return stat.healthy ? 'ok' : 'err';
}

export default function DashboardPanel() {
  const { qverisKeys, customProviders, setTotals } = useRouterStore();

  // Real data state
  const [stats, setStats]           = useState<RouterStats | null>(null);
  const [health, setHealth]         = useState<RouterHealth | null>(null);
  const [culiOnline, setCuliOnline] = useState<boolean | null>(null);
  const [loading, setLoading]       = useState(true);
  const [lastRefresh, setLastRefresh] = useState<Date | null>(null);
  const [activity, setActivity]     = useState<number[]>([]);

  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  async function fetchAll() {
    try {
      const [s, h] = await Promise.all([getRouterStats(), getRouterHealth()]);
      setStats(s);
      setHealth(h);
      // Sync store totals from real data
      setTotals({
        totalRequests: s.router.requestCount,
        totalTokens: 0, // Router doesn't track tokens — keep local
        totalFailovers: s.router.failoverCount,
      });
      setLastRefresh(new Date());
    } catch {
      // Router offline — keep previous data
    }
    try {
      await getCuliHealth();
      setCuliOnline(true);
    } catch {
      setCuliOnline(false);
    }
    setLoading(false);
  }

  // Build activity bars from requestCount over refreshes
  const reqHistory = useRef<number[]>([]);
  useEffect(() => {
    fetchAll();
    intervalRef.current = setInterval(() => {
      fetchAll();
    }, 5000); // refresh every 5s
    return () => { if (intervalRef.current) clearInterval(intervalRef.current); };
  }, []);

  useEffect(() => {
    if (!stats) return;
    const cur = stats.router.requestCount;
    reqHistory.current = [...reqHistory.current.slice(-13), cur];
    // Normalise to percentage of max for bar chart
    const max = Math.max(...reqHistory.current, 1);
    setActivity(reqHistory.current.map(v => Math.round((v / max) * 92 + 8)));
  }, [stats]);

  // ─── Derived values ──────────────────────────────────────────────────────
  const activeKeys = qverisKeys.filter(k => k.active).length
    + customProviders.reduce((a, p) => a + p.keys.filter(k => k.active).length, 0);

  const reqCount   = stats?.router.requestCount  ?? 0;
  const failCount  = stats?.router.failoverCount ?? 0;
  const successRate = stats
    ? parseFloat(stats.router.successRate) / 100
    : null;
  const uptimeSec = health?.uptime ?? null;

  const blackboxStat = stats?.providers.blackbox;
  const sixthStat    = stats?.providers.sixth;
  const qverisStat   = stats?.providers.qveris;
  const freeHealthy  = (blackboxStat?.healthy || sixthStat?.healthy) ?? false;
  const freeReqs     = (blackboxStat?.totalRequests ?? 0) + (sixthStat?.totalRequests ?? 0);

  const routerOnline = health !== null;

  return (
    <div className="dp">
      {/* ── Status bar ───────────────────────────────────────────────── */}
      <div className="dp-status-bar">
        <span className={`dp-svc-dot ${routerOnline ? 'dp-svc-ok' : 'dp-svc-err'}`}>
          {routerOnline ? <Wifi size={11} /> : <WifiOff size={11} />}
          Router {routerOnline ? 'online' : 'offline'}
          {routerOnline && <span className="dp-svc-port">:4000</span>}
        </span>
        <span className={`dp-svc-dot ${culiOnline ? 'dp-svc-ok' : culiOnline === false ? 'dp-svc-err' : 'dp-svc-dim'}`}>
          {culiOnline ? <Wifi size={11} /> : <WifiOff size={11} />}
          CULI backend {culiOnline ? 'online' : culiOnline === false ? 'offline' : '…'}
          {culiOnline && <span className="dp-svc-port">:3111</span>}
        </span>
        <span className="dp-svc-time">
          {loading ? 'Loading…' : lastRefresh ? `Updated ${lastRefresh.toLocaleTimeString()}` : ''}
        </span>
        <button className="dp-refresh-btn" onClick={fetchAll} title="Refresh">
          <RefreshCw size={12} className={loading ? 'dp-spin' : ''} />
        </button>
      </div>

      {/* ── KPI cards ────────────────────────────────────────────────── */}
      <section className="dp-grid">
        <article className="dp-card">
          <div className="dp-card-h">
            <span className="dp-card-ic dp-ic-accent"><Activity size={16} /></span>
            <span className="dp-card-lbl">Total Requests</span>
          </div>
          <div className="dp-card-v">{loading ? '—' : fmtInt(reqCount)}</div>
          <div className="dp-card-d">
            <span className="dp-tag dp-tag-good">live</span>
            <span className="dp-dim">from router /stats</span>
          </div>
        </article>

        <article className="dp-card">
          <div className="dp-card-h">
            <span className="dp-card-ic dp-ic-accent"><CheckCircle2 size={16} /></span>
            <span className="dp-card-lbl">Success Rate</span>
          </div>
          <div className="dp-card-v">
            {loading ? '—' : successRate !== null ? fmtPct(successRate) : 'N/A'}
          </div>
          <div className="dp-card-d">
            <span className={`dp-tag ${successRate === null || successRate >= 0.95 ? 'dp-tag-good' : 'dp-tag-warn'}`}>
              <CheckCircle2 size={10} />
              {successRate !== null ? (successRate >= 0.95 ? 'healthy' : 'degraded') : '—'}
            </span>
            <span className="dp-dim">target 98.5%</span>
          </div>
        </article>

        <article className="dp-card">
          <div className="dp-card-h">
            <span className="dp-card-ic dp-ic-warn"><RefreshCw size={16} /></span>
            <span className="dp-card-lbl">Failover Count</span>
          </div>
          <div className="dp-card-v">{loading ? '—' : fmtInt(failCount)}</div>
          <div className="dp-card-d">
            <span className={`dp-tag ${failCount === 0 ? 'dp-tag-good' : 'dp-tag-warn'}`}>
              <AlertCircle size={10} />
              {reqCount > 0 ? fmtPct(failCount / reqCount) : '0.0%'}
            </span>
            <span className="dp-dim">of requests</span>
          </div>
        </article>

        <article className="dp-card">
          <div className="dp-card-h">
            <span className="dp-card-ic dp-ic-accent"><Shield size={16} /></span>
            <span className="dp-card-lbl">Active Keys</span>
          </div>
          <div className="dp-card-v">{activeKeys}</div>
          <div className="dp-card-d">
            <span className="dp-tag dp-tag-accent"><Server size={10} /> live</span>
            <span className="dp-dim">across providers</span>
          </div>
        </article>

        <article className="dp-card">
          <div className="dp-card-h">
            <span className="dp-card-ic dp-ic-ok"><Clock size={16} /></span>
            <span className="dp-card-lbl">Router Uptime</span>
          </div>
          <div className="dp-card-v">
            {loading ? '—' : uptimeSec !== null ? fmtUptime(uptimeSec) : 'N/A'}
          </div>
          <div className="dp-card-d">
            <span className={`dp-tag ${routerOnline ? 'dp-tag-good' : 'dp-tag-warn'}`}>
              <CheckCircle2 size={10} /> {routerOnline ? 'running' : 'offline'}
            </span>
            <span className="dp-dim">since last restart</span>
          </div>
        </article>

        <article className="dp-card">
          <div className="dp-card-h">
            <span className="dp-card-ic dp-ic-accent"><Cpu size={16} /></span>
            <span className="dp-card-lbl">Free Requests</span>
          </div>
          <div className="dp-card-v">{loading ? '—' : fmtInt(freeReqs)}</div>
          <div className="dp-card-d">
            <span className="dp-tag dp-tag-good">$0</span>
            <span className="dp-dim">Blackbox + Sixth</span>
          </div>
        </article>
      </section>

      {/* ── Provider Health ──────────────────────────────────────────── */}
      <section className="dp-sec">
        <div className="dp-sec-h">
          <h3 className="dp-sec-t">Provider Health</h3>
          <span className="dp-sec-s">
            {routerOnline ? 'Live · polling 5s' : 'Router offline — cached data'}
          </span>
        </div>

        <div className="dp-pv-row">
          {/* CULI FreeModel (Blackbox + Sixth) */}
          <article className={`dp-pv dp-pv-${freeHealthy ? 'ok' : 'err'}`}>
            <div className="dp-pv-h">
              <ProviderLogo displayName="CULI Free" provider="blackbox" modelName="sixth" size={28} />
              <div>
                <strong className="dp-pv-name">CULI FreeModel</strong>
                <span className="dp-pv-sub">
                  Blackbox {blackboxStat?.healthy ? '✓' : '✗'} · Sixth {sixthStat?.healthy ? '✓' : '✗'}
                </span>
              </div>
              <span className={`dp-dot dp-dot-${freeHealthy ? 'ok' : 'err'}`} />
            </div>
            <div className="dp-pv-m">
              <div className="dp-pv-m-i">
                <span className="dp-dim">requests</span>
                <strong>{fmtInt(freeReqs)}</strong>
              </div>
              <div className="dp-pv-m-i">
                <span className="dp-dim">errors</span>
                <strong className={freeHealthy ? 'dp-ok' : 'dp-err'}>
                  {fmtInt((blackboxStat?.totalErrors ?? 0) + (sixthStat?.totalErrors ?? 0))}
                </strong>
              </div>
              <div className="dp-pv-m-i">
                <span className="dp-dim">pool</span>
                <strong>{sixthStat?.poolSize ?? '—'}</strong>
              </div>
            </div>
            <div className="dp-pv-bar">
              <span
                className={`dp-pv-bar-f dp-pv-bar-${freeHealthy ? 'ok' : 'err'}`}
                style={{ width: freeHealthy ? '100%' : '0%' }}
              />
            </div>
          </article>

          {/* Qveris */}
          <article className={`dp-pv dp-pv-${providerStatus(qverisStat)}`}>
            <div className="dp-pv-h">
              <ProviderLogo displayName="Qveris" provider="qveris" size={28} />
              <div>
                <strong className="dp-pv-name">Qveris</strong>
                <span className="dp-pv-sub">
                  {qverisStat?.modelCount ?? '—'} models ·{' '}
                  {qverisStat?.hasApiKey ? 'key set' : 'no key'}
                </span>
              </div>
              <span className={`dp-dot dp-dot-${providerStatus(qverisStat)}`} />
            </div>
            <div className="dp-pv-m">
              <div className="dp-pv-m-i">
                <span className="dp-dim">requests</span>
                <strong>{fmtInt(qverisStat?.totalRequests ?? 0)}</strong>
              </div>
              <div className="dp-pv-m-i">
                <span className="dp-dim">errors</span>
                <strong>{fmtInt(qverisStat?.totalErrors ?? 0)}</strong>
              </div>
              <div className="dp-pv-m-i">
                <span className="dp-dim">credits</span>
                <strong>
                  {qverisStat?.remainingCredits != null
                    ? qverisStat.remainingCredits.toFixed(2)
                    : '—'}
                </strong>
              </div>
            </div>
            <div className="dp-pv-bar">
              <span
                className={`dp-pv-bar-f dp-pv-bar-${providerStatus(qverisStat)}`}
                style={{ width: qverisStat?.healthy ? '100%' : '0%' }}
              />
            </div>
          </article>

          {/* Custom providers */}
          <article className={`dp-pv dp-pv-${culiOnline ? 'ok' : 'err'}`}>
            <div className="dp-pv-h">
              <span className="dp-pv-ic"><Zap size={16} /></span>
              <div>
                <strong className="dp-pv-name">Custom</strong>
                <span className="dp-pv-sub">{customProviders.length} providers · {activeKeys} keys</span>
              </div>
              <span className={`dp-dot dp-dot-${activeKeys > 0 ? 'ok' : 'warn'}`} />
            </div>
            <div className="dp-pv-m">
              <div className="dp-pv-m-i">
                <span className="dp-dim">providers</span>
                <strong>{customProviders.length}</strong>
              </div>
              <div className="dp-pv-m-i">
                <span className="dp-dim">active keys</span>
                <strong>{activeKeys}</strong>
              </div>
              <div className="dp-pv-m-i">
                <span className="dp-dim">models</span>
                <strong>{customProviders.reduce((a, p) => a + p.models.length, 0)}</strong>
              </div>
            </div>
            <div className="dp-pv-bar">
              <span
                className="dp-pv-bar-f dp-pv-bar-ok"
                style={{ width: activeKeys > 0 ? '100%' : '20%' }}
              />
            </div>
          </article>
        </div>
      </section>

      {/* ── Activity chart ───────────────────────────────────────────── */}
      <section className="dp-sec">
        <div className="dp-sec-h">
          <h3 className="dp-sec-t">Request Activity</h3>
          <span className="dp-sec-s">
            {activity.length > 1 ? `${activity.length} samples · live` : 'Accumulating…'}
          </span>
        </div>
        <article className="dp-act">
          <div className="dp-act-bars" style={{ gridTemplateColumns: `repeat(${Math.max(activity.length, 14)}, 1fr)` }}>
            {(activity.length > 0 ? activity : Array(14).fill(8)).map((h, i) => (
              <div key={i} className="dp-act-col" style={{ height: `${h}%` }} title={`${h}%`} />
            ))}
          </div>
          <div className="dp-act-x">
            <span>oldest</span>
            <span>now</span>
          </div>
        </article>
      </section>

      <style>{`
        .dp { display:flex; flex-direction:column; gap:var(--space-lg); padding:var(--space-md) 0; }

        /* Status bar */
        .dp-status-bar {
          display:flex; align-items:center; gap:var(--space-sm); flex-wrap:wrap;
          padding:6px 12px; background:var(--color-surface);
          border:1px solid var(--color-rule); border-radius:var(--radius-md);
          font-size:var(--text-xs); font-family:'Geist Mono',monospace;
        }
        .dp-svc-dot { display:inline-flex; align-items:center; gap:4px; padding:2px 8px; border-radius:var(--radius-sm); border:1px solid var(--color-rule); }
        .dp-svc-ok  { color:var(--color-ink-2); }
        .dp-svc-err { color:var(--color-muted); opacity:.6; }
        .dp-svc-dim { color:var(--color-muted); }
        .dp-svc-port { opacity:.5; margin-left:2px; }
        .dp-svc-time { margin-left:auto; color:var(--color-muted); }
        .dp-refresh-btn {
          display:grid; place-items:center; padding:4px; border-radius:var(--radius-sm);
          background:var(--color-surface); border:1px solid var(--color-rule);
          color:var(--color-ink-2); cursor:pointer;
          transition:background .15s;
        }
        .dp-refresh-btn:hover { background:var(--color-paper-2); }
        @keyframes spin { to { transform:rotate(360deg); } }
        .dp-spin { animation:spin .8s linear infinite; }

        /* KPI grid */
        .dp-grid { display:grid; grid-template-columns:repeat(6,1fr); gap:var(--space-sm); }
        @media (max-width:1200px) { .dp-grid { grid-template-columns:repeat(3,1fr); } }
        @media (max-width:720px)  { .dp-grid { grid-template-columns:repeat(2,1fr); } }

        .dp-card {
          background:var(--color-paper-2); border:1px solid var(--color-rule);
          border-radius:var(--radius-lg); padding:var(--space-md);
          display:flex; flex-direction:column; gap:var(--space-xs);
        }
        .dp-card-h  { display:flex; align-items:center; gap:var(--space-xs); }
        .dp-card-lbl{ font-size:var(--text-xs); color:var(--color-muted); font-weight:600; text-transform:uppercase; letter-spacing:.06em; }
        .dp-card-v  { font-family:var(--font-body); font-size:var(--text-lg); font-weight:700; color:var(--color-ink); line-height:var(--leading-tight); }
        .dp-card-d  { display:flex; align-items:center; gap:var(--space-xs); font-size:var(--text-xs); }
        .dp-card-ic { width:28px; height:28px; display:grid; place-items:center; border-radius:var(--radius-md); }
        .dp-ic-accent,.dp-ic-ok,.dp-ic-warn,.dp-ic-err {
          background:var(--color-surface); border:1px solid var(--color-rule); color:var(--color-ink-2);
        }
        .dp-tag { display:inline-flex; align-items:center; gap:3px; padding:1px 6px; border-radius:var(--radius-sm); font-size:var(--text-xs); font-weight:600; font-family:'Geist Mono',monospace; }
        .dp-tag-good   { background:var(--color-surface); border:1px solid var(--color-rule); color:var(--color-ink-2); font-weight:700; }
        .dp-tag-warn   { background:var(--color-surface); border:1px solid var(--color-rule); color:var(--color-muted); font-weight:500; }
        .dp-tag-accent { background:var(--color-surface); border:1px solid var(--color-rule); color:var(--color-ink-2); font-weight:700; }
        .dp-dim { color:var(--color-muted); font-size:var(--text-xs); }
        .dp-ok  { color:var(--color-ink-2); }
        .dp-warn{ color:var(--color-muted); }
        .dp-err { color:var(--color-muted); opacity:.7; }

        /* Section */
        .dp-sec { display:flex; flex-direction:column; gap:var(--space-sm); }
        .dp-sec-h { display:flex; align-items:baseline; justify-content:space-between; }
        .dp-sec-t { font-family:var(--font-body); font-size:var(--text-md); font-weight:600; color:var(--color-ink); }
        .dp-sec-s { font-size:var(--text-xs); color:var(--color-muted); font-family:'Geist Mono',monospace; }

        /* Provider health */
        .dp-pv-row { display:grid; grid-template-columns:repeat(3,1fr); gap:var(--space-sm); }
        @media (max-width:900px) { .dp-pv-row { grid-template-columns:1fr; } }
        .dp-pv {
          background:var(--color-paper-2); border:1px solid var(--color-rule);
          border-radius:var(--radius-lg); padding:var(--space-md);
          display:flex; flex-direction:column; gap:var(--space-sm);
        }
        .dp-pv-ok  { border-color:var(--color-rule); }
        .dp-pv-warn{ border-color:var(--color-rule); }
        .dp-pv-err { border-color:var(--color-rule); opacity:.75; }
        .dp-pv-h   { display:flex; align-items:center; gap:var(--space-sm); }
        .dp-pv-ic  {
          width:32px; height:32px; display:grid; place-items:center;
          background:var(--color-surface); border:1px solid var(--color-rule);
          color:var(--color-ink-2); border-radius:var(--radius-md); flex-shrink:0;
        }
        .dp-pv-name { display:block; font-size:var(--text-sm); font-weight:600; color:var(--color-ink); }
        .dp-pv-sub  { display:block; font-size:var(--text-xs); color:var(--color-muted); }
        .dp-pv-h > :nth-child(3) { margin-left:auto; }
        .dp-dot { width:8px; height:8px; border-radius:50%; display:inline-block; }
        .dp-dot-ok  { background:var(--color-ink); box-shadow:0 0 8px color-mix(in srgb,var(--color-ink) 40%,transparent); }
        .dp-dot-warn{ background:var(--color-ink); box-shadow:0 0 8px color-mix(in srgb,var(--color-ink) 25%,transparent); }
        .dp-dot-err { background:var(--color-muted); }
        .dp-pv-m { display:grid; grid-template-columns:repeat(3,1fr); gap:var(--space-xs); }
        .dp-pv-m-i { display:flex; flex-direction:column; gap:2px; }
        .dp-pv-m-i span   { font-size:var(--text-xs); color:var(--color-muted); font-family:'Geist Mono',monospace; text-transform:uppercase; letter-spacing:.05em; }
        .dp-pv-m-i strong { font-size:var(--text-sm); color:var(--color-ink); font-variant-numeric:tabular-nums; }
        .dp-pv-bar { height:4px; background:var(--color-surface); border-radius:2px; overflow:hidden; }
        .dp-pv-bar-f { display:block; height:100%; border-radius:2px; transition:width .4s ease; }
        .dp-pv-bar-ok  { background:var(--color-ink); }
        .dp-pv-bar-warn{ background:var(--color-ink); opacity:.6; }
        .dp-pv-bar-err { background:var(--color-muted); }

        /* Activity */
        .dp-act {
          background:var(--color-paper-2); border:1px solid var(--color-rule);
          border-radius:var(--radius-lg); padding:var(--space-md);
          display:flex; flex-direction:column; gap:var(--space-sm);
        }
        .dp-act-bars { display:grid; gap:var(--space-xs); align-items:end; height:140px; }
        .dp-act-col {
          background:linear-gradient(180deg,var(--color-ink),color-mix(in srgb,var(--color-ink) 50%,transparent));
          border-radius:3px 3px 1px 1px; min-height:6px;
          transition:transform .18s ease, filter .18s ease;
        }
        .dp-act-col:hover { filter:brightness(1.15); transform:scaleY(1.04); transform-origin:bottom; }
        .dp-act-x { display:flex; justify-content:space-between; font-size:var(--text-xs); color:var(--color-muted); font-family:'Geist Mono',monospace; }
      `}</style>
    </div>
  );
}
