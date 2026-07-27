import React, { useState } from 'react';
import { AlertTriangle, Search } from 'lucide-react';

export function KnowledgeGraphTab() {
  const [selectedNode, setSelectedNode] = useState<{ name: string; type: string; loc: number; callers: number } | null>({
    name: 'CULI Orchestrator Engine',
    type: 'Rust Module (orchestrator/engine.rs)',
    loc: 480,
    callers: 12
  });

  const GRAPH_NODES = [
    { id: 1, name: 'main.rs', type: 'Entry', x: 20, y: 30, color: '#3b82f6', god: false },
    { id: 2, name: 'orchestrator/engine.rs', type: 'God Node', x: 50, y: 50, color: '#ef4444', god: true },
    { id: 3, name: 'memory/search.rs', type: 'Module', x: 80, y: 30, color: '#10b981', god: false },
    { id: 4, name: 'provider/router.rs', type: 'Module', x: 30, y: 70, color: '#f59e0b', god: false },
    { id: 5, name: 'skills/executor.rs', type: 'Module', x: 70, y: 75, color: '#ec4899', god: false },
  ];

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-sm)', height: '100%' }}>
      {/* Graph Query Bar */}
      <div style={{ display: 'flex', gap: '6px' }}>
        <div style={{ position: 'relative', flex: 1 }}>
          <input
            type="text"
            placeholder="Query graph (e.g. path orchestrator memory)"
            defaultValue="explain orchestrator/engine"
            style={{
              width: '100%',
              padding: '4px 8px 4px 26px',
              fontSize: '11px',
              background: 'var(--color-paper)',
              border: '1px solid var(--color-rule)',
              borderRadius: '4px',
              color: 'var(--color-ink)'
            }}
          />
          <Search size={12} style={{ position: 'absolute', left: '8px', top: '7px', color: 'var(--color-muted)' }} />
        </div>
        <button className="mode-btn active" style={{ fontSize: '10px' }}>Query</button>
      </div>

      {/* Interactive Graph Canvas Preview */}
      <div className="graph-canvas-wrapper">
        <svg width="100%" height="100%" style={{ position: 'absolute', inset: 0 }}>
          {/* Edges */}
          <line x1="20%" y1="30%" x2="50%" y2="50%" stroke="var(--color-rule)" strokeWidth="1.5" />
          <line x1="50%" y1="50%" x2="80%" y2="30%" stroke="var(--color-rule)" strokeWidth="1.5" strokeDasharray="4 2" />
          <line x1="50%" y1="50%" x2="30%" y2="70%" stroke="var(--color-rule)" strokeWidth="1.5" />
          <line x1="50%" y1="50%" x2="70%" y2="75%" stroke="var(--color-rule)" strokeWidth="1.5" />
        </svg>

        {GRAPH_NODES.map((n) => (
          <div
            key={n.id}
            onClick={() => setSelectedNode({ name: n.name, type: n.type, loc: n.god ? 480 : 120, callers: n.god ? 12 : 3 })}
            style={{
              position: 'absolute',
              left: `${n.x}%`,
              top: `${n.y}%`,
              transform: 'translate(-50%, -50%)',
              padding: '4px 8px',
              borderRadius: '12px',
              background: n.color,
              color: '#ffffff',
              fontSize: '9px',
              fontWeight: 600,
              cursor: 'pointer',
              zIndex: 10
            }}
            className={n.god ? 'god-node-glow' : ''}
          >
            {n.name}
          </div>
        ))}
      </div>

      {/* God Node Alert */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '6px', background: 'rgba(239, 68, 68, 0.1)', border: '1px solid rgba(239, 68, 68, 0.3)', padding: '6px 8px', borderRadius: '4px', fontSize: '10px', color: '#ef4444' }}>
        <AlertTriangle size={12} />
        <span><strong>God Node Detected:</strong> <code>orchestrator/engine.rs</code> high coupling (12 inbound callers).</span>
      </div>

      {/* Node Inspector Drawer */}
      {selectedNode && (
        <div style={{ background: 'var(--color-paper-3)', border: '1px solid var(--color-rule)', borderRadius: '6px', padding: '8px', fontSize: '11px', display: 'flex', flexDirection: 'column', gap: '4px' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', fontWeight: 600 }}>
            <span>{selectedNode.name}</span>
            <span className="badge-role architect">{selectedNode.type}</span>
          </div>
          <div style={{ display: 'flex', gap: '12px', color: 'var(--color-muted)', fontSize: '10px' }}>
            <span>Lines of Code: <strong>{selectedNode.loc}</strong></span>
            <span>Callers: <strong>{selectedNode.callers}</strong></span>
          </div>
        </div>
      )}
    </div>
  );
}
