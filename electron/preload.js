const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('electronAPI', {
  platform:   process.platform,
  version:    process.env.npm_package_version || '0.1.0',

  // App controls
  quit:       () => ipcRenderer.invoke('quit'),
  showWindow: () => ipcRenderer.invoke('show-window'),

  // Project folder picker — opens native dialog
  pickFolder: () => ipcRenderer.invoke('pick-folder'),
});
