import { useEffect, useRef, useState } from 'react';
import { RefreshCw, ZoomIn, ZoomOut, Download, Box, WifiOff } from 'lucide-react';

const API = import.meta.env.VITE_API_URL || 'http://localhost:3111/api';

interface ArchNode {
  id: string;
  label: string;
  type: 'container' | 'component' | 'actor' | 'external';
  layer: string;
  description?: string;
  technology?: string;
  x?: number;
  y?: number;
}

interface ArchEdge {
  from: string;
  to:   string;
  label?: string;
}

interface ArchData {
  nodes: ArchNode[];
  edges: ArchEdge[];
  generated_at: string;
}

// Static architecture data (mirrors culi.c4 — updated when /api/graph/c4 available)
const STATIC_ARCH: ArchData = {
  generated_at: '2026-07-26',
  nodes: [
    { id: 'user',         label: 'User',            type: 'actor',     layer: 'external',  description: 'Developer using CULI' },
    { id: 'electron',     label: 'Electron Shell',  type: 'container', layer: 'shell',     technology: 'Electron v31', description: 'Desktop wrapper. Spawns backend.' },
    { id: 'frontend',     label: 'React Frontend',  type: 'container', layer: 'frontend',  technology: 'React + Vite', description: 'Chat UI, Router panel, Visualizer' },
    { id: 'backend',      label: 'Rust Backend',    type: 'container', layer: 'backend',   technology: 'Axum 0.8', description: 'API server :3111, agent engine' },
    { id: 'orchestrator', label: 'Orchestrator',    type: 'component', layer: 'backend',   description: 'Agent loop + task decomposition' },
    { id: 'provider',     label: 'Provider Router', type: 'component', layer: 'backend',   description: 'Routes: qveris → sixth → blackbox → ollama' },
    { id: 'memory',       label: 'Memory Pipeline', type: 'component', layer: 'backend',   description: 'Working/Episodic/Semantic/Procedural' },
    { id: 'tools',        label: 'Tool Registry',   type: 'component', layer: 'backend',   description: '7 tools: filesystem, terminal, web_search...' },
    { id: 'skills',       label: 'Skill Loader',    type: 'component', layer: 'backend',   description: 'Loads agent brains from skills/*.md' },
    { id: 'qveris',       label: 'CULI Models',     type: 'container', layer: 'provider',  technology: 'Qveris API', description: 'User-visible models (Wangsu + OpenRouter)' },
    { id: 'harness',      label: 'Harness Layer',   type: 'container', layer: 'provider',  technology: 'Sixth AI + Blackbox', description: 'Free, internal only. Hidden from user.' },
    { id: 'db',           label: 'SQLite + Tantivy', type: 'container', layer: 'storage',  description: 'data/culi/memory.db + search index' },
  ],
  edges: [
    { from: 'user',      to: 'electron',     label: 'launches' },
    { from: 'electron',  to: 'frontend',     label: 'loads file://' },
    { from: 'electron',  to: 'backend',      label: 'spawns --serve' },
    { from: 'frontend',  to: 'backend',      label: 'HTTP localhost:3111' },
    { from: 'backend',   to: 'orchestrator', label: 'routes chat' },
    { from: 'orchestrator', to: 'provider',  label: 'LLM calls' },
    { from: 'orchestrator', to: 'tools',     label: 'execute' },
    { from: 'orchestrator', to: 'memory',    label: 'R/W context' },
    { from: 'orchestrator', to: 'skills',    label: 'load brain' },
    { from: 'provider',  to: 'qveris',       label: 'CULI models' },
    { from: 'provider',  to: 'harness',      label: 'internal tasks' },
    { from: 'memory',    to: 'db',           label: 'persist' },
  ],
};

// Layer color mapping
const LAYER_COLORS: Record<string, { bg: string; border: string; text: string }> = {
  external:  { bg: '#1a1a2e', border: '#444', text: '#aaa' },
  shell:     { bg: '#1a2535', border: '#4a6fa5', text: '#8ab4e8' },
  frontend:  { bg: '#1a2a1a', border: '#4a8a4a', text: '#8ae88a' },
  backend:   { bg: '#2a1a1a', border: '#8a4a4a', text: '#e88a8a' },
  provider:  { bg: '#2a2a1a', border: '#8a8a4a', text: '#e8e88a' },
  storage:   { bg: '#1a1a1a', border: '#666', text: '#aaa' },
};

export function ArchitectureTab() {
  const [arch, setArch]       = useState<ArchData>(STATIC_ARCH);
  const [loading, setLoading] = useState(false);
  const [error, setError]     = useState<string | null>(null);
  const [selected, setSelected] = useState<ArchNode | null>(null);
  const [zoom, setZoom]       = useState(1);

  const fetchArch = async () => {
    setLoading(true);
    try {
      const r = await fetch(`${API}/graph/c4`);
      if (r.ok) {
        const data = await r.json();
        setArch(data);
        setError(null);
      }
    } catch {
      setError('Using static diagram — /api/graph/c4 not available yet');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { fetchArch(); }, []);

  // Simple force-directed layout (static positions by layer)
  const layerPositions: Record<string, { col: number; order: number }> = {};
  const layerGroups: Record<string, ArchNode[]> = {};
  arch.nodes.forEach(n => {
    if (!layerGroups[n.layer]) layerGroups[n.layer] = [];
    layerGroups[n.layer].push(n);
  });

  const layerOrder = ['external', 'shell', 'frontend', 'backend', 'provider', 'storage'];
  const COL_W = 220, ROW_H = 100, PAD_X = 40, PAD_Y = 60;

  layerOrder.forEach((layer, col) => {
    (layerGroups[layer] || []).forEach((node, row) => {
      node.x = PAD_X + col * COL_W;
      node.y = PAD_Y + row * ROW_H;
    });
  });

  const svgW = PAD_X * 2 + layerOrder.length * COL_W;
  const svgH = PAD_Y * 2 + Math.max(...Object.values(layerGroups).map(g => g.length)) * ROW_H;

  const nodeById = Object.fromEntries(arch.nodes.map(n => [n.id, n]));

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden' }}>
      {/* Toolbar */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 6, padding: '6px 10px', borderBottom: '1px solid var(--color-rule)', flexShrink: 0 }}>
        <Box size={13} />
        <strong style={{ fontSize: 11 }}>Architecture Map</strong>
        {error && <span style={{ fontSize: 10, color: 'var(--color-muted)', display: 'flex', alignItems: 'center', gap: 4 }}><WifiOff size={10} /> {error}</span>}
        <div style={{ marginLeft: 'auto', display: 'flex', gap: 4 }}>
          <button className="icon-btn" onClick={() => setZoom(z => Math.min(z + 0.2, 2))}   title="Zoom in">  <ZoomIn  size={12} /></button>
          <button className="icon-btn" onClick={() => setZoom(z => Math.max(z - 0.2, 0.4))} title="Zoom out"><ZoomOut size={12} /></button>
          <button className="icon-btn" onClick={() => setZoom(1)} title="Reset">1:1</button>
          <button className="icon-btn" onClick={fetchArch} title="Refresh"><RefreshCw size={12} className={loading ? 'dag-spin' : ''} /></button>
        </div>
      </div>

      <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
        {/* SVG diagram */}
        <div style={{ flex: 1, overflow: 'auto', background: 'var(--color-paper-1)' }}>
          <svg
            width={svgW * zoom}
            height={svgH * zoom}
            viewBox={`0 0 ${svgW} ${svgH}`}
            style={{ display: 'block', minWidth: '100%' }}
          >
            {/* Layer labels */}
            {layerOrder.map((layer, col) => {
              const colors = LAYER_COLORS[layer] || LAYER_COLORS.storage;
              const nodes = layerGroups[layer] || [];
              if (!nodes.length) return null;
              const x = PAD_X + col * COL_W - 10;
              const h = nodes.length * ROW_H + 20;
              return (
                <g key={layer}>
                  <rect x={x} y={PAD_Y - 20} width={180} height={h}
                    fill={colors.bg} stroke={colors.border} strokeWidth="1" rx="6" opacity="0.5" />
                  <text x={x + 90} y={PAD_Y - 6} textAnchor="middle"
                    fontSize="9" fill={colors.text} fontWeight="600" textTransform="uppercase">
                    {layer.toUpperCase()}
                  </text>
                </g>
              );
            })}

            {/* Edges */}
            {arch.edges.map((edge, i) => {
              const from = nodeById[edge.from];
              const to   = nodeById[edge.to];
              if (!from?.x || !to?.x) return null;
              const mx = (from.x + to.x) / 2;
              const my = (from.y! + to.y!) / 2;
              return (
                <g key={i}>
                  <line
                    x1={from.x + 80} y1={from.y! + 20}
                    x2={to.x + 80}   y2={to.y! + 20}
                    stroke="#444" strokeWidth="1" markerEnd="url(#arrow)"
                  />
                  {edge.label && (
                    <text x={mx + 80} y={my + 15} fontSize="8" fill="#666" textAnchor="middle">
                      {edge.label}
                    </text>
                  )}
                </g>
              );
            })}

            {/* Arrow marker */}
            <defs>
              <marker id="arrow" markerWidth="6" markerHeight="6" refX="6" refY="3" orient="auto">
                <path d="M0,0 L0,6 L6,3 z" fill="#444" />
              </marker>
            </defs>

            {/* Nodes */}
            {arch.nodes.map(node => {
              if (node.x === undefined) return null;
              const colors = LAYER_COLORS[node.layer] || LAYER_COLORS.storage;
              const isSelected = selected?.id === node.id;
              return (
                <g key={node.id} style={{ cursor: 'pointer' }}
                  onClick={() => setSelected(isSelected ? null : node)}>
                  <rect
                    x={node.x} y={node.y!}
                    width={160} height={50}
                    rx="6"
                    fill={isSelected ? colors.border : 'var(--color-surface)'}
                    stroke={isSelected ? '#fff' : colors.border}
                    strokeWidth={isSelected ? 2 : 1}
                  />
                  <text x={node.x + 80} y={node.y! + 17} textAnchor="middle"
                    fontSize="11" fontWeight="600" fill={colors.text}>
                    {node.label}
                  </text>
                  {node.technology && (
                    <text x={node.x + 80} y={node.y! + 30} textAnchor="middle"
                      fontSize="8" fill="#666">
                      {node.technology}
                    </text>
                  )}
                  <text x={node.x + 80} y={node.y! + 43} textAnchor="middle"
                    fontSize="7" fill="#555">
                    {node.type}
                  </text>
                </g>
              );
            })}
          </svg>
        </div>

        {/* Detail panel */}
        {selected && (
          <div style={{
            width: 220, borderLeft: '1px solid var(--color-rule)',
            padding: 12, fontSize: 11, flexShrink: 0, overflow: 'auto',
          }}>
            <div style={{ fontWeight: 700, marginBottom: 6, color: 'var(--color-ink)' }}>{selected.label}</div>
            <div style={{ color: 'var(--color-muted)', marginBottom: 4 }}>
              <span style={{ fontWeight: 600 }}>Layer: </span>{selected.layer}
            </div>
            {selected.technology && (
              <div style={{ color: 'var(--color-muted)', marginBottom: 4 }}>
                <span style={{ fontWeight: 600 }}>Tech: </span>{selected.technology}
              </div>
            )}
            {selected.description && (
              <div style={{ color: 'var(--color-ink-2)', marginTop: 8, lineHeight: 1.5 }}>
                {selected.description}
              </div>
            )}
            <div style={{ marginTop: 12, fontSize: 10, color: 'var(--color-muted)' }}>
              Connected to:
              {arch.edges
                .filter(e => e.from === selected.id || e.to === selected.id)
                .map((e, i) => (
                  <div key={i} style={{ padding: '2px 0' }}>
                    {e.from === selected.id ? `→ ${nodeById[e.to]?.label}` : `← ${nodeById[e.from]?.label}`}
                    {e.label && <span style={{ color: '#555' }}> ({e.label})</span>}
                  </div>
                ))}
            </div>
          </div>
        )}
      </div>

      <style>{`@keyframes spin { to { transform: rotate(360deg); } } .dag-spin { animation: spin .8s linear infinite; display: inline-block; }`}</style>
    </div>
  );
}
