import React, { useEffect, useRef, useState } from 'react';
import {
  Bot, Check, ChevronDown, ChevronRight, Clock, Cpu,
  Play, RefreshCw, ZoomIn, ZoomOut, Wrench, WifiOff, Database,
} from 'lucide-react';

const API = import.meta.env.VITE_API_URL || 'http://localhost:3111/api';

interface TraceNode {
  id: string;
  role: 'architect' | 'security' | 'dev' | 'memory' | 'provider';
  label: string;
  status: 'running' | 'completed' | 'pending' | 'error';
  duration_ms: number;
  tokens?: number;
  tool?: string | null;
  thinking?: string;
}

interface TraceData {
  session_uptime_seconds: number;
  memory: {
    working: number;
    episodic: number;
    semantic: number;
    procedural: number;
    total: number;
  };
  nodes: TraceNode[];
}

function statusIcon(s: TraceNode['status']) {
  switch (s) {
    case 'completed': return <Check size={10} />;
    case 'running':   return <Play size={10} />;
    case 'error':     return <WifiOff size={10} />;
    default:          return <Clock size={10} />;
  }
}

function statusColor(s: TraceNode['status']) {
  switch (s) {
    case 'completed': return 'var(--color-success)';
    case 'running':   return 'var(--color-accent)';
    case 'error':     return 'var(--color-muted)';
    default:          return 'var(--color-muted)';
  }
}

function formatMs(ms: number) {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${Math.floor(ms / 60000)}m ${Math.floor((ms % 60000) / 1000)}s`;
}

export function DagTraceTab() {
  const [data, setData]     = useState<TraceData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError]   = useState<string | null>(null);
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  async function fetchTrace() {
    try {
      const res = await fetch(`${API}/trace`);
      if (!res.ok) throw new Error(`${res.status}`);
      const json: TraceData = await res.json();
      setData(json);
      setError(null);
      // Auto-expand first node
      if (json.nodes.length > 0 && Object.keys(expanded).length === 0) {
        setExpanded({ [json.nodes[0].id]: true });
      }
    } catch (e) {
      setError('CULI backend offline');
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    fetchTrace();
    intervalRef.current = setInterval(fetchTrace, 5000);
    return () => { if (intervalRef.current) clearInterval(intervalRef.current); };
  }, []);

  const toggle = (id: string) =>
    setExpanded(prev => ({ ...prev, [id]: !prev[id] }));

  return (
    <div className="dag-container">
      {/* Toolbar */}
      <div className="dag-toolbar">
        <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
          <Cpu size={14} className="accent" />
          <strong style={{ fontSize: 11 }}>Orchestrator Execution DAG</strong>
          {data && (
            <span style={{ fontSize: 10, color: 'var(--color-muted)', fontFamily: 'var(--font-mono)' }}>
              · uptime {formatMs(data.session_uptime_seconds * 1000)}
            </span>
          )}
        </div>
        <div style={{ display: 'flex', gap: 4 }}>
          <button className="icon-btn" title="Zoom In"><ZoomIn size={12} /></button>
          <button className="icon-btn" title="Zoom Out"><ZoomOut size={12} /></button>
          <button className="icon-btn" title="Refresh" onClick={fetchTrace}>
            <RefreshCw size={12} className={loading ? 'dag-spin' : ''} />
          </button>
        </div>
      </div>

      {/* Memory bar */}
      {data && (
        <div className="dag-membar">
          <Database size={11} />
          <span>Memory:</span>
          <span className="dag-mem-chip">W<strong>{data.memory.working}</strong></span>
          <span className="dag-mem-chip">E<strong>{data.memory.episodic}</strong></span>
          <span className="dag-mem-chip">S<strong>{data.memory.semantic}</strong></span>
          <span className="dag-mem-chip">P<strong>{data.memory.procedural}</strong></span>
          <span className="dag-mem-total">total: {data.memory.total}</span>
        </div>
      )}

      {/* Offline / loading */}
      {error && (
        <div className="dag-offline">
          <WifiOff size={14} /> {error} — showing cached data
        </div>
      )}

      {loading && !data && (
        <div className="dag-loading">
          <RefreshCw size={14} className="dag-spin" /> Loading trace…
        </div>
      )}

      {/* DAG Nodes */}
      {(data?.nodes ?? []).map((node, idx) => (
        <React.Fragment key={node.id}>
          <div className={`dag-node ${node.status === 'running' ? 'active' : ''}`}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                <span className={`badge-role ${node.role}`}>{node.role}</span>
                <strong style={{ fontSize: 12 }}>{node.label}</strong>
              </div>
              <span style={{ fontSize: 10, color: statusColor(node.status), display: 'flex', alignItems: 'center', gap: 4 }}>
                {statusIcon(node.status)}
                {node.status.toUpperCase()}
                {node.duration_ms > 0 && ` (${formatMs(node.duration_ms)})`}
              </span>
            </div>

            {/* Tool */}
            {node.tool && (
              <div style={{ fontSize: 11, color: 'var(--color-muted)', display: 'flex', alignItems: 'center', gap: 6 }}>
                <Wrench size={12} /> Tool: <code>{node.tool}</code>
              </div>
            )}

            {/* Thinking block */}
            {node.thinking && (
              <div className="thinking-block">
                <div className="thinking-header" onClick={() => toggle(node.id)}>
                  <span style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                    <Bot size={12} />
                    {node.tokens
                      ? `Context: ${node.tokens.toLocaleString()} tokens`
                      : 'Internal state'}
                  </span>
                  {expanded[node.id] ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
                </div>
                {expanded[node.id] && (
                  <div className="thinking-content">
                    {node.thinking.split('\n').map((line, i) => (
                      <span key={i}>{line}<br /></span>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>

          {/* Connector (not after last node) */}
          {idx < (data?.nodes.length ?? 0) - 1 && <div className="dag-connector" />}
        </React.Fragment>
      ))}

      <style>{`
        @keyframes spin { to { transform: rotate(360deg); } }
        .dag-spin { animation: spin .8s linear infinite; display: inline-block; }
        .dag-membar {
          display: flex; align-items: center; gap: 6px;
          padding: 6px 10px; margin: 6px 0;
          background: var(--color-surface); border: 1px solid var(--color-rule);
          border-radius: var(--radius-md);
          font-size: var(--text-xs); color: var(--color-muted);
          font-family: var(--font-mono);
        }
        .dag-mem-chip { display: inline-flex; gap: 3px; padding: 1px 6px; background: var(--color-paper-2); border-radius: 4px; font-size: 10px; }
        .dag-mem-chip strong { color: var(--color-ink); }
        .dag-mem-total { margin-left: auto; color: var(--color-ink-2); font-weight: 600; }
        .dag-offline {
          display: flex; align-items: center; gap: 6px;
          padding: 8px 12px; background: var(--color-surface);
          border: 1px solid var(--color-rule); border-radius: var(--radius-md);
          font-size: var(--text-xs); color: var(--color-muted);
        }
        .dag-loading {
          display: flex; align-items: center; gap: 8px;
          padding: 16px; color: var(--color-muted); font-size: var(--text-sm);
        }
      `}</style>
    </div>
  );
}
