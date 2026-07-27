import React, { useEffect, useRef, useState } from 'react';
import { Clock, Plus, UserCheck, RefreshCw, WifiOff } from 'lucide-react';

const API = import.meta.env.VITE_API_URL || 'http://localhost:3111/api';

interface KanbanTask {
  id: string;
  title: string;
  subagent: string;
  role: 'architect' | 'security' | 'dev' | 'memory' | 'provider' | 'tester';
  column: 'backlog' | 'in_progress' | 'review' | 'done';
  progress: number;
  time: string;
}

interface TasksData {
  tasks: KanbanTask[];
  columns: string[];
  uptime_seconds: number;
}

const COLUMNS: { id: KanbanTask['column']; name: string }[] = [
  { id: 'backlog',     name: 'Backlog'         },
  { id: 'in_progress', name: 'In Progress'     },
  { id: 'review',      name: 'Agent Review'    },
  { id: 'done',        name: 'Done'            },
];

function roleColor(role: string) {
  switch (role) {
    case 'architect': return 'var(--color-accent)';
    case 'security':  return 'var(--color-ink-2)';
    case 'memory':    return 'var(--color-ink-2)';
    case 'provider':  return 'var(--color-ink-2)';
    case 'tester':    return 'var(--color-muted)';
    default:          return 'var(--color-ink-2)';
  }
}

export function KanbanTab() {
  const [data, setData]       = useState<TasksData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError]     = useState<string | null>(null);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  async function fetchTasks() {
    try {
      const res = await fetch(`${API}/tasks`);
      if (!res.ok) throw new Error(`${res.status}`);
      const json: TasksData = await res.json();
      setData(json);
      setError(null);
    } catch {
      setError('CULI backend offline');
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    fetchTasks();
    intervalRef.current = setInterval(fetchTasks, 6000);
    return () => { if (intervalRef.current) clearInterval(intervalRef.current); };
  }, []);

  const tasks = data?.tasks ?? [];

  return (
    <div>
      {/* Toolbar */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 8, padding: '4px 0' }}>
        <span style={{ fontSize: 11, color: 'var(--color-muted)', fontFamily: 'var(--font-mono)' }}>
          {data ? `${tasks.length} tasks · uptime ${data.uptime_seconds}s · live` : 'Loading…'}
        </span>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
          {error && (
            <span style={{ display: 'flex', alignItems: 'center', gap: 4, fontSize: 11, color: 'var(--color-muted)' }}>
              <WifiOff size={11} /> {error}
            </span>
          )}
          <button className="icon-btn" onClick={fetchTasks} title="Refresh">
            <RefreshCw size={12} className={loading ? 'kb-spin' : ''} />
          </button>
        </div>
      </div>

      <div className="kanban-board">
        {COLUMNS.map(col => {
          const colTasks = tasks.filter(t => t.column === col.id);
          return (
            <div key={col.id} className="kanban-col">
              <div className="kanban-col-title">
                <span>{col.name} ({colTasks.length})</span>
                <button className="icon-btn" title="Add Task"><Plus size={12} /></button>
              </div>

              {loading && colTasks.length === 0 && (
                <div className="kanban-placeholder">
                  <RefreshCw size={12} className="kb-spin" style={{ color: 'var(--color-muted)' }} />
                </div>
              )}

              {colTasks.map(task => (
                <div key={task.id} className="kanban-card">
                  <strong style={{ color: 'var(--color-ink)', fontSize: 11 }}>
                    {task.title}
                  </strong>

                  <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', margin: '2px 0' }}>
                    <span className={`badge-role ${task.role}`} style={{ color: roleColor(task.role) }}>
                      <UserCheck size={9} /> {task.subagent}
                    </span>
                    <span style={{ fontSize: 9, color: 'var(--color-muted)', display: 'flex', alignItems: 'center', gap: 2 }}>
                      <Clock size={9} /> {task.time}
                    </span>
                  </div>

                  {/* Progress bar */}
                  <div style={{ width: '100%', height: 3, background: 'var(--color-rule)', borderRadius: 2, overflow: 'hidden' }}>
                    <div style={{
                      width: `${task.progress}%`,
                      height: '100%',
                      background: task.progress === 100
                        ? 'var(--color-success, var(--color-ink-2))'
                        : 'var(--color-accent)',
                      transition: 'width .4s ease',
                    }} />
                  </div>
                </div>
              ))}

              {/* Empty state */}
              {!loading && colTasks.length === 0 && (
                <div className="kanban-empty">no tasks</div>
              )}
            </div>
          );
        })}
      </div>

      <style>{`
        @keyframes spin { to { transform: rotate(360deg); } }
        .kb-spin { animation: spin .8s linear infinite; display: inline-block; }
        .kanban-placeholder { display: flex; justify-content: center; padding: 16px; }
        .kanban-empty { font-size: 11px; color: var(--color-muted); text-align: center; padding: 12px; font-family: var(--font-mono); }
      `}</style>
    </div>
  );
}
