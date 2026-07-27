// --- Review Mode for Web Projects ---
// This is a simplified version that works in a browser environment using localStorage

const reviewState = {
  initialized: false,
  config: {
    enabled: true,
    outputPath: '.uiux-web-feedback.json',
    draftPath: '.uiux-web-feedback.draft.json',
  },
  visible: true,
  mode: 'browse',
  selected: null,
  comments: [],
  editingCommentId: null,
  commentDraft: {
    text: '',
    tags: '',
    severity: 'minor',
    attachment: null,
    attachedFiles: [],
  },
  nextCommentId: 1,
  status: {
    text: '',
    tone: 'muted',
  },
  drag: {
    isDragging: false,
    offsetX: 0,
    offsetY: 0,
  },
  position: {
    x: 10,
    y: 10,
  },
};

function escapeHtml(value) {
  return String(value ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function escapeAttr(value) {
  return escapeHtml(value).replace(/`/g, '&#96;');
}

function getReviewRoot() {
  return document.getElementById('af-review-root');
}

function getMarkersRoot() {
  return document.getElementById('af-review-markers');
}

function isReviewUi(target) {
  const root = getReviewRoot();
  return !!(root && target instanceof Element && root.contains(target));
}

function generateSelector(node) {
  if (node.id) return `#${CSS.escape(node.id)}`;
  if (node === document.body) return 'body';
  if (node === document.documentElement) return 'html';

  const parts = [];
  let current = node;

  while (current && current !== document.body && current !== document.documentElement) {
    let segment = current.tagName.toLowerCase();
    if (current.id) {
      segment += `#${CSS.escape(current.id)}`;
      parts.unshift(segment);
      break;
    }

    const classNames = Array.from(current.classList || [])
      .filter((className) => className && !className.startsWith('af-review'))
      .slice(0, 2)
      .map((className) => `.${CSS.escape(className)}`)
      .join('');

    if (classNames) {
      segment += classNames;
    }

    const parent = current.parentElement;
    if (parent) {
      const sameTagSiblings = Array.from(parent.children).filter(
        (child) => child.tagName === current.tagName,
      );
      if (sameTagSiblings.length > 1) {
        segment += `:nth-of-type(${sameTagSiblings.indexOf(current) + 1})`;
      }
    }

    parts.unshift(segment);
    current = current.parentElement;
  }

  return parts.join(' > ');
}

function buildPageContext() {
  return {
    page: document.title,
    path: window.location.pathname + window.location.search + window.location.hash,
    url: window.location.href,
    userAgent: navigator.userAgent,
    viewport: {
      width: window.innerWidth,
      height: window.innerHeight,
    }
  };
}

function buildTarget(selection) {
  const nodeId = selection.node?.id || '';
  return {
    selector: selection.selector,
    tag: selection.tag,
    text_hint: selection.text.slice(0, 160),
    id_hint: nodeId || undefined,
  };
}

function resetCommentDraft() {
  reviewState.commentDraft = {
    text: '',
    tags: '',
    severity: 'minor',
    attachment: null,
    attachedFiles: [],
  };
}

function captureSelection(node) {
  const rect = node.getBoundingClientRect();
  return {
    node,
    selector: generateSelector(node),
    tag: node.tagName.toLowerCase(),
    text: (node.textContent || '').trim().slice(0, 300),
    classes: Array.from(node.classList || []).filter((className) => !className.startsWith('af-review')),
    rect: {
      left: rect.left,
      top: rect.top,
      width: rect.width,
      height: rect.height,
    },
  };
}

function serializeSelection(selection = reviewState.selected) {
  if (!selection) return null;
  return {
    selector: selection.selector,
    tag: selection.tag,
    text: selection.text,
    id_hint: selection.node?.id || '',
    sizeLabel: `${Math.round(selection.rect.width)} x ${Math.round(selection.rect.height)}`,
  };
}

function buildDraftPayload() {
  return {
    version: '2.2',
    app: 'CULI Agent',
    visible: reviewState.visible,
    mode: reviewState.mode,
    selectedSelector: reviewState.selected?.selector || '',
    comments: reviewState.comments.map(c => ({ ...c })),
    commentDraft: { ...reviewState.commentDraft },
    nextCommentId: reviewState.nextCommentId,
    editingCommentId: reviewState.editingCommentId,
    position: reviewState.position,
    savedAt: new Date().toISOString(),
  };
}

function setStatus(text, tone = 'muted') {
  reviewState.status = { text, tone };
  renderReviewUi();
  // Auto hide status after 3 seconds
  setTimeout(() => {
    if (reviewState.status.text === text) {
      reviewState.status = { text: '', tone: 'muted' };
      renderReviewUi();
    }
  }, 3000);
}

function renderMarkers() {
  const container = getMarkersRoot();
  if (!container) return;

  if (!reviewState.visible) {
    container.innerHTML = '';
    return;
  }

  const comments = reviewState.comments;
  container.innerHTML = comments.map((record, index) => {
    let node = null;
    try {
      node = document.querySelector(record.selector);
    } catch (e) { /* ignore */ }

    if (!node) return '';

    const rect = node.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) return '';

    const top = Math.max(10, rect.top - 10 + ((index % 3) * 22));
    const left = Math.max(10, rect.left + rect.width - 18);
    return `
      <button class="af-review-marker" type="button" data-marker-comment="${record.id}" style="top:${top}px;left:${left}px;" title="${escapeHtml(record.instruction || record.selector)}">
        ${index + 1}
      </button>
    `;
  }).join('');
}

function ensureReviewUi() {
  if (getReviewRoot()) return;

  const root = document.createElement('div');
  root.id = 'af-review-root';
  root.innerHTML = `
    <div id="af-review-markers" class="af-review-markers"></div>
    <div id="af-review-highlight" class="af-review-highlight" aria-hidden="true"></div>
    <div id="af-review-panel" class="af-review-panel">
      <div class="af-review-panel-header" id="af-review-drag-handle">
        <div class="af-review-panel-title">Review Panel</div>
        <button id="af-review-close-btn" class="af-review-btn">&times;</button>
      </div>
      <div class="af-review-panel-content">
        <!-- Content will be rendered here -->
      </div>
    </div>
    <button id="af-review-toggle-btn" class="af-review-toggle-btn">💬</button>
  `;
  document.body.appendChild(root);

  const panel = document.getElementById('af-review-panel');
  const dragHandle = document.getElementById('af-review-drag-handle');

  // Set initial position
  panel.style.left = `${reviewState.position.x}px`;
  panel.style.top = `${reviewState.position.y}px`;

  // Attach drag event listeners
  dragHandle.addEventListener('mousedown', (e) => {
    reviewState.drag.isDragging = true;
    reviewState.drag.offsetX = e.clientX - reviewState.position.x;
    reviewState.drag.offsetY = e.clientY - reviewState.position.y;
    dragHandle.style.cursor = 'grabbing';
    e.preventDefault();
  });

  document.addEventListener('mousemove', (e) => {
    if (!reviewState.drag.isDragging) return;
    reviewState.position.x = e.clientX - reviewState.drag.offsetX;
    reviewState.position.y = e.clientY - reviewState.drag.offsetY;
    panel.style.left = `${reviewState.position.x}px`;
    panel.style.top = `${reviewState.position.y}px`;
    saveDraftToLocalStorage();
  });

  document.addEventListener('mouseup', () => {
    if (reviewState.drag.isDragging) {
      reviewState.drag.isDragging = false;
      dragHandle.style.cursor = 'grab';
    }
  });

  // Attach other event listeners
  root.addEventListener('click', onReviewClick);
  document.getElementById('af-review-close-btn').addEventListener('click', () => toggleReviewUi(false));
  document.getElementById('af-review-toggle-btn').addEventListener('click', () => toggleReviewUi());
  document.addEventListener('click', handleSelection, true);
  document.addEventListener('keydown', handleKeydown, true);
  window.addEventListener('resize', renderReviewUi);
  window.addEventListener('scroll', renderReviewUi, true);
}

async function onReviewClick(event) {
  const markerId = event.target.dataset.markerComment;
  if (markerId) {
    event.preventDefault();
    event.stopPropagation();
    focusSelectionForRecord(Number(markerId));
    return;
  }

  if (event.target.dataset.reviewMode) {
    setMode(event.target.dataset.reviewMode);
  }
}

function hideHighlight() {
  const highlight = document.getElementById('af-review-highlight');
  if (highlight) {
    highlight.style.display = 'none';
  }
}

function showHighlightForNode(node) {
  const highlight = document.getElementById('af-review-highlight');
  if (!highlight || !reviewState.visible) return;

  const rect = node.getBoundingClientRect();
  highlight.style.display = 'block';
  highlight.style.left = `${rect.left}px`;
  highlight.style.top = `${rect.top}px`;
  highlight.style.width = `${rect.width}px`;
  highlight.style.height = `${rect.height}px`;
}

function clearSelection() {
  reviewState.selected = null;
  resetCommentDraft();
  hideHighlight();
  renderReviewUi();
}

function setMode(mode) {
  reviewState.mode = mode === 'comment' ? 'comment' : 'browse';
  hideHighlight();
  renderReviewUi();
}

function toggleReviewUi(show = null) {
  reviewState.visible = show !== null ? show : !reviewState.visible;
  const panel = document.getElementById('af-review-panel');
  const toggleBtn = document.getElementById('af-review-toggle-btn');
  if (panel) panel.style.display = reviewState.visible ? 'flex' : 'none';
  if (toggleBtn) toggleBtn.style.display = reviewState.visible ? 'none' : 'block';
  hideHighlight();
  renderReviewUi();
  saveDraftToLocalStorage();
}

function renderContext(snapshot) {
  const page = snapshot?.pageContext || {};
  return `
    <div class="af-panel-section">
      <div class="af-panel-section__title">Context</div>
      <div class="af-panel-grid af-panel-grid--meta">
        <div class="af-panel-meta"><span>Page</span><strong>${escapeHtml(page.page || 'Unknown')}</strong></div>
        <div class="af-panel-meta"><span>URL</span><strong>${escapeHtml(page.path || '/')}</strong></div>
      </div>
    </div>
  `;
}

function renderSelection(snapshot) {
  const selected = snapshot?.selected;
  if (!selected) {
    return `
      <div class="af-panel-section">
        <div class="af-panel-section__title">Selected Element</div>
        <div class="af-panel-empty">Select an element on the page in Comment mode.</div>
      </div>
    `;
  }
  return `
    <div class="af-panel-section">
      <div class="af-panel-section__title">Selected Element</div>
      <div class="af-panel-card">
        <div class="af-panel-selector">${escapeHtml(selected.selector)}</div>
        <div class="af-panel-badges">
          <span>${escapeHtml(selected.tag)}</span>
          <span>${escapeHtml(selected.sizeLabel)}</span>
          ${selected.id_hint ? `<span>#${escapeHtml(selected.id_hint)}</span>` : ''}
        </div>
        <div class="af-panel-text">${escapeHtml(selected.text || '(no text)')}</div>
      </div>
    </div>
  `;
}

function renderCommentEditor(snapshot) {
  const draft = getMergedCommentDraft(snapshot);
  const isEditing = snapshot?.editingCommentId !== null;
  const severityLabels = {
    critical: 'Critical',
    major: 'Major',
    minor: 'Minor',
    suggestion: 'Suggestion'
  };
  return `
    <div class="af-panel-section">
      <div class="af-panel-section__title">${isEditing ? `Editing Comment #${snapshot.editingCommentId}` : 'New Comment'}</div>
      <div class="af-panel-form">
        <div class="af-panel-help">Write clear instructions for fixing this element.</div>
        <label class="af-panel-field">
          <span>Instruction</span>
          <textarea id="af-panel-comment-text" rows="5" placeholder="Describe the fix...">${escapeHtml(draft.text || '')}</textarea>
        </label>
        <label class="af-panel-field">
          <span>Tags (comma separated)</span>
          <input id="af-panel-comment-tags" type="text" value="${escapeAttr(draft.tags || '')}" placeholder="layout, typography">
        </label>
        <label class="af-panel-field">
          <span>Severity</span>
          <select id="af-panel-comment-severity">
            ${['critical', 'major', 'minor', 'suggestion']
              .map((severity) => `<option value="${severity}"${draft.severity === severity ? ' selected' : ''}>${severityLabels[severity]}</option>`)
              .join('')}
          </select>
        </label>
        <div class="af-panel-actions">
          <button data-command="add-comment" class="af-panel-btn af-panel-btn--primary">${isEditing ? 'Update Comment' : 'Add Comment'}</button>
          ${isEditing ? `<button data-command="cancel-edit" class="af-panel-btn">Cancel</button>` : `<button data-command="clear-selection" class="af-panel-btn">Clear</button>`}
        </div>
      </div>
    </div>
  `;
}

function renderActionSection(snapshot) {
  if (snapshot?.mode === 'comment') return renderCommentEditor(snapshot);
  return `
    <div class="af-panel-section">
      <div class="af-panel-section__title">Mode</div>
      <div class="af-panel-empty">Browse mode: normal app usage. Switch to Comment mode to select elements.</div>
    </div>
  `;
}

function renderRecord(record, index) {
  const preview = record.attachment?.dataUrl
    ? `<img class="af-panel-record__thumb" src="${escapeAttr(record.attachment.dataUrl)}" alt="Attachment">`
    : '';
  const context = record.context || {};
  const contextText = [context.page, context.path].filter(Boolean).join(' | ');
  return `
    <article class="af-panel-record">
      <div class="af-panel-record__head">
        <span class="af-panel-pill af-panel-pill--comment">#${index + 1}</span>
        <div class="af-panel-record__actions">
          <button data-edit-comment="${record.id}" class="af-panel-record-btn">Edit</button>
          <button data-delete-comment="${record.id}" class="af-panel-delete">Delete</button>
        </div>
      </div>
      ${preview}
      <div class="af-panel-record__selector">${escapeHtml(record.target?.selector || record.selector || '')}</div>
      <div class="af-panel-record__body">${escapeHtml(record.instruction || '(no instruction)')}</div>
      <div class="af-panel-record__meta">${escapeHtml(record.severity || 'minor')} | ${escapeHtml((record.tags || []).join(', ') || 'no tags')}</div>
      <div class="af-panel-record__meta">${escapeHtml(contextText || 'no context')}</div>
    </article>
  `;
}

function renderRecords(snapshot) {
  const records = [...(snapshot?.comments || [])].sort((a, b) => new Date(b.timestamp || 0) - new Date(a.timestamp || 0));
  if (records.length === 0) {
    return `
      <div class="af-panel-section af-panel-section--grow">
        <div class="af-panel-section__title">Comments (${records.length})</div>
        <div class="af-panel-empty">No comments yet.</div>
      </div>
    `;
  }
  return `
    <div class="af-panel-section af-panel-section--grow">
      <div class="af-panel-section__title">
        <span>Comments (${records.length})</span>
        <button data-command="clear-all-comments" class="af-panel-btn af-panel-btn--danger">Clear All</button>
      </div>
      <div class="af-panel-records">
        ${records.map((r, idx) => renderRecord(r, idx)).join('')}
      </div>
    </div>
  `;
}

function getMergedCommentDraft(snapshot) {
  return {
    text: '',
    tags: '',
    severity: 'minor',
    attachment: null,
    attachedFiles: [],
    ...(snapshot?.commentDraft || {}),
    ...(reviewState.localCommentDraft || {}),
  };
}

function buildSnapshot() {
  return {
    visible: reviewState.visible,
    mode: reviewState.mode,
    status: reviewState.status,
    selected: serializeSelection(),
    comments: reviewState.comments,
    editingCommentId: reviewState.editingCommentId,
    commentDraft: { ...reviewState.commentDraft },
    pageContext: buildPageContext(),
    config: reviewState.config,
  };
}

function renderReviewUi() {
  const targetPanelContent = document.querySelector('#af-review-panel .af-review-panel-content');
  if (!targetPanelContent) return;
  
  renderMarkers();
  
  const snapshot = buildSnapshot();
  targetPanelContent.innerHTML = `
    <div class="af-panel-toolbar">
      <div class="af-panel-mode-group">
        <button data-command="set-mode" data-mode="browse" class="af-panel-btn${snapshot.mode === 'browse' ? ' is-active' : ''}">Browse</button>
        <button data-command="set-mode" data-mode="comment" class="af-panel-btn${snapshot.mode === 'comment' ? ' is-active' : ''}">Comment</button>
      </div>
      <div class="af-panel-toolbar__actions">
        <button data-command="export-feedback" class="af-panel-btn af-panel-btn--primary">Export</button>
      </div>
    </div>
    ${snapshot.status.text ? `<div class="af-panel-status" data-tone="${escapeAttr(snapshot.status.tone)}">${escapeHtml(snapshot.status.text)}</div>` : ''}
    ${renderContext(snapshot)}
    ${renderSelection(snapshot)}
    ${renderActionSection(snapshot)}
    ${renderRecords(snapshot)}
  `;
  
  // Re-attach event listeners to new elements
  targetPanelContent.querySelectorAll('[data-command]').forEach(btn => {
    btn.addEventListener('click', (e) => {
      handlePanelCommand({ type: e.target.dataset.command, ...e.target.dataset });
    });
  });
  
  targetPanelContent.querySelectorAll('[data-delete-comment]').forEach(btn => {
    btn.addEventListener('click', (e) => {
      handlePanelCommand({ type: 'delete-comment', id: Number(e.target.dataset.deleteComment) });
    });
  });
  
  targetPanelContent.querySelectorAll('[data-edit-comment]').forEach(btn => {
    btn.addEventListener('click', (e) => {
      handlePanelCommand({ type: 'edit-comment', id: Number(e.target.dataset.editComment) });
    });
  });
  
  // Attach input listeners
  const commentText = document.getElementById('af-panel-comment-text');
  if (commentText) {
    commentText.addEventListener('input', (e) => {
      reviewState.commentDraft.text = e.target.value;
      saveDraftToLocalStorage();
    });
    commentText.value = reviewState.commentDraft.text;
  }
  
  const commentTags = document.getElementById('af-panel-comment-tags');
  if (commentTags) {
    commentTags.addEventListener('input', (e) => {
      reviewState.commentDraft.tags = e.target.value;
      saveDraftToLocalStorage();
    });
    commentTags.value = reviewState.commentDraft.tags;
  }
  
  const commentSeverity = document.getElementById('af-panel-comment-severity');
  if (commentSeverity) {
    commentSeverity.addEventListener('change', (e) => {
      reviewState.commentDraft.severity = e.target.value;
      saveDraftToLocalStorage();
    });
    commentSeverity.value = reviewState.commentDraft.severity;
  }
}

function handleSelection(event) {
  if (!reviewState.visible || reviewState.mode === 'browse') return;
  if (!(event.target instanceof Element) || isReviewUi(event.target)) return;

  event.preventDefault();
  event.stopPropagation();

  reviewState.selected = captureSelection(event.target);
  showHighlightForNode(event.target);
  setStatus(`Selected: ${reviewState.selected.selector}`, 'info');
  renderReviewUi();
  saveDraftToLocalStorage();
}

async function addCommentRecord() {
  if (!reviewState.selected && !reviewState.editingCommentId) {
    setStatus('Select an element first.', 'warn');
    return;
  }

  const tags = (reviewState.commentDraft.tags || '')
    .split(/[,\n]/)
    .map((tag) => tag.trim())
    .filter(Boolean);

  if (!reviewState.commentDraft.text.trim()) {
    setStatus('Please write an instruction.', 'warn');
    return;
  }

  if (reviewState.editingCommentId !== null) {
    const index = reviewState.comments.findIndex((c) => c.id === reviewState.editingCommentId);
    if (index !== -1) {
      const updatedRecord = {
        ...reviewState.comments[index],
        instruction: reviewState.commentDraft.text.trim(),
        severity: reviewState.commentDraft.severity,
        tags,
        timestamp: new Date().toISOString(),
      };
      if (reviewState.selected && reviewState.selected.selector !== updatedRecord.selector) {
        updatedRecord.selector = reviewState.selected.selector;
        updatedRecord.target = buildTarget(reviewState.selected);
        updatedRecord.context = buildPageContext();
      }
      reviewState.comments[index] = updatedRecord;
      setStatus('Comment updated.', 'success');
    }
  } else {
    reviewState.comments.unshift({
      id: reviewState.nextCommentId++,
      selector: reviewState.selected.selector,
      target: buildTarget(reviewState.selected),
      context: buildPageContext(),
      instruction: reviewState.commentDraft.text.trim(),
      severity: reviewState.commentDraft.severity,
      tags,
      timestamp: new Date().toISOString(),
    });
    setStatus('Comment added.', 'success');
  }

  reviewState.editingCommentId = null;
  resetCommentDraft();
  renderReviewUi();
  saveDraftToLocalStorage();
}

function focusSelectionForRecord(id) {
  const record = reviewState.comments.find((item) => item.id === id);
  if (!record) return;
  
  const node = document.querySelector(record.selector);
  if (!node) {
    setStatus('Element not found on current page.', 'warn');
    return;
  }
  
  reviewState.selected = captureSelection(node);
  showHighlightForNode(node);
  renderReviewUi();
  setStatus(`Focused on ${record.selector}`, 'info');
  saveDraftToLocalStorage();
}

async function handlePanelCommand(command = {}) {
  switch (command.type) {
    case 'set-mode':
      if (command.mode) setMode(command.mode);
      break;
    case 'clear-selection':
      clearSelection();
      break;
    case 'add-comment':
      await addCommentRecord();
      break;
    case 'delete-comment':
      reviewState.comments = reviewState.comments.filter((item) => item.id !== Number(command.id));
      if (reviewState.editingCommentId === Number(command.id)) {
        reviewState.editingCommentId = null;
        resetCommentDraft();
      }
      renderReviewUi();
      saveDraftToLocalStorage();
      setStatus('Comment deleted.', 'info');
      break;
    case 'edit-comment': {
      const record = reviewState.comments.find((item) => item.id === Number(command.id));
      if (record) {
        reviewState.editingCommentId = record.id;
        reviewState.commentDraft = {
          text: record.instruction || '',
          tags: (record.tags || []).join(', '),
          severity: record.severity || 'minor',
          attachment: null,
          attachedFiles: [],
        };
        focusSelectionForRecord(record.id);
        setMode('comment');
      }
      break;
    }
    case 'cancel-edit':
      reviewState.editingCommentId = null;
      resetCommentDraft();
      renderReviewUi();
      break;
    case 'clear-all-comments':
      if (confirm('Delete all comments? This cannot be undone.')) {
        reviewState.comments = [];
        reviewState.nextCommentId = 1;
        reviewState.editingCommentId = null;
        resetCommentDraft();
        renderReviewUi();
        saveDraftToLocalStorage();
        setStatus('All comments cleared.', 'info');
      }
      break;
    case 'export-feedback':
      await exportFeedback();
      break;
    default:
      break;
  }
}

function handleKeydown(event) {
  if (event.ctrlKey && event.shiftKey && event.key.toLowerCase() === 'r') {
    event.preventDefault();
    toggleReviewUi();
    return;
  }
  
  if (!reviewState.visible) return;
  
  if (event.ctrlKey && event.key.toLowerCase() === 's') {
    event.preventDefault();
    exportFeedback();
    return;
  }
  
  if (event.target instanceof HTMLElement) {
    const tagName = event.target.tagName;
    if (tagName === 'INPUT' || tagName === 'TEXTAREA' || tagName === 'SELECT') {
      return;
    }
  }
  
  const key = event.key.toLowerCase();
  if (key === 'b') setMode('browse');
  else if (key === 'c') setMode('comment');
  else if (key === 'escape') clearSelection();
}

function saveDraftToLocalStorage() {
  try {
    localStorage.setItem('af-review-draft', JSON.stringify(buildDraftPayload()));
  } catch (e) {
    console.error('Failed to save draft:', e);
  }
}

function loadDraftFromLocalStorage() {
  try {
    const raw = localStorage.getItem('af-review-draft');
    if (raw) {
      const draft = JSON.parse(raw);
      reviewState.visible = draft.visible !== false;
      reviewState.mode = draft.mode === 'comment' ? 'comment' : 'browse';
      reviewState.comments = draft.comments || [];
      reviewState.commentDraft = draft.commentDraft || reviewState.commentDraft;
      reviewState.nextCommentId = draft.nextCommentId || 1;
      reviewState.editingCommentId = draft.editingCommentId || null;
      if (draft.position) {
        reviewState.position = draft.position;
      }
    }
  } catch (e) {
    console.error('Failed to load draft:', e);
  }
}

async function exportFeedback() {
  const pageContexts = reviewState.comments.map(record => record.context).filter(Boolean);
  const uniquePages = Array.from(new Map(pageContexts.map(item => [`${item.page}|${item.path || ''}`, item])).values());
  
  const payload = {
    version: '2.2',
    app: 'CULI Agent',
    comments: reviewState.comments.map(record => ({
      id: record.id,
      target: record.target,
      context: record.context,
      instruction: record.instruction,
      severity: record.severity,
      tags: record.tags,
      timestamp: record.timestamp,
    })),
    pages: uniquePages,
    savedAt: new Date().toISOString(),
  };

  const dataStr = JSON.stringify(payload, null, 2);
  const dataBlob = new Blob([dataStr], { type: 'application/json' });
  const url = URL.createObjectURL(dataBlob);
  const a = document.createElement('a');
  a.href = url;
  a.download = 'culi-agent-feedback.json';
  a.click();
  URL.revokeObjectURL(url);
  
  setStatus('Feedback exported!', 'success');
}

export async function setupInAppReview() {
  if (reviewState.initialized) return;
  
  loadDraftFromLocalStorage();
  ensureReviewUi();
  
  // Apply loaded position
  const panel = document.getElementById('af-review-panel');
  if (panel) {
    panel.style.left = `${reviewState.position.x}px`;
    panel.style.top = `${reviewState.position.y}px`;
  }
  
  renderReviewUi();
  reviewState.initialized = true;
  setStatus('Review mode enabled!', 'info');
}
