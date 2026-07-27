const { app, BrowserWindow, Tray, Menu, nativeImage, ipcMain, dialog } = require('electron');
const { spawn } = require('child_process');
const path = require('path');
const net  = require('net');
const fs   = require('fs');

let mainWindow = null;
let tray       = null;
let backendProcess = null;

// ── Backend path ─────────────────────────────────────────────────────
function getBackendPath() {
  if (app.isPackaged) {
    // Packaged: extraResources/backend/culi.exe
    return path.join(process.resourcesPath, 'backend', 'culi.exe');
  }
  // Dev: cargo build output
  if (process.env.CULI_DEV_MODE === 'true') {
    return path.join(__dirname, '..', 'target', 'debug', 'culi.exe');
  }
  return path.join(__dirname, '..', 'target', 'release', 'culi.exe');
}

// ── Frontend path ────────────────────────────────────────────────────
function getFrontendPath() {
  if (app.isPackaged) {
    return path.join(process.resourcesPath, 'frontend', 'index.html');
  }
  return path.join(__dirname, '..', 'frontend', 'dist', 'index.html');
}

// ── Wait until port is open ───────────────────────────────────────────
function waitForPort(port, retries = 30) {
  return new Promise((resolve, reject) => {
    let attempts = 0;
    const check = () => {
      const sock = net.createConnection(port, '127.0.0.1');
      sock.once('connect', () => { sock.destroy(); resolve(); });
      sock.once('error', () => {
        sock.destroy();
        if (++attempts >= retries) return reject(new Error(`Port ${port} not ready`));
        setTimeout(check, 500);
      });
    };
    check();
  });
}

// ── Spawn CULI backend ────────────────────────────────────────────────
function spawnBackend() {
  const backendPath = getBackendPath();
  if (!fs.existsSync(backendPath)) {
    console.warn('[electron] backend not found:', backendPath);
    return;
  }

  console.log('[electron] spawning backend:', backendPath);
  backendProcess = spawn(backendPath, ['--serve'], {
    cwd: path.dirname(backendPath),
    stdio: ['ignore', 'pipe', 'pipe'],
    detached: false,
  });

  backendProcess.stdout.on('data', d => console.log('[backend]', d.toString().trim()));
  backendProcess.stderr.on('data', d => console.warn('[backend]', d.toString().trim()));
  backendProcess.on('exit', code => console.log('[backend] exited:', code));
}

// ── Create main window ────────────────────────────────────────────────
function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1400, height: 900,
    minWidth: 900, minHeight: 600,
    show: false,
    icon: path.join(__dirname, '..', 'icons', 'icon.ico'),
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
    },
    titleBarStyle: 'default',
    frame: true,
  });

  // Load frontend
  if (process.env.CULI_VITE_DEV === 'true') {
    mainWindow.loadURL('http://localhost:5173');
    mainWindow.webContents.openDevTools();
  } else {
    mainWindow.loadFile(getFrontendPath());
  }

  mainWindow.once('ready-to-show', () => {
    mainWindow.show();
  });

  // X button → minimize to tray
  mainWindow.on('close', (e) => {
    e.preventDefault();
    mainWindow.hide();
  });
}

// ── System tray ───────────────────────────────────────────────────────
function createTray() {
  const iconPath = path.join(__dirname, '..', 'icons', '32x32.png');
  const icon = nativeImage.createFromPath(iconPath);
  tray = new Tray(icon);

  const menu = Menu.buildFromTemplate([
    { label: 'Open CULI', click: () => { mainWindow.show(); mainWindow.focus(); } },
    { type: 'separator' },
    { label: 'Quit', click: () => {
        mainWindow.destroy();
        if (backendProcess) backendProcess.kill();
        app.quit();
    }},
  ]);

  tray.setToolTip('CULI Agent');
  tray.setContextMenu(menu);
  tray.on('click', () => { mainWindow.show(); mainWindow.focus(); });
}

// ── IPC Handlers ─────────────────────────────────────────────────────
ipcMain.handle('quit', () => {
  if (backendProcess) backendProcess.kill();
  app.exit(0);
});

ipcMain.handle('show-window', () => {
  if (mainWindow) { mainWindow.show(); mainWindow.focus(); }
});

ipcMain.handle('pick-folder', async () => {
  const result = await dialog.showOpenDialog(mainWindow, {
    properties: ['openDirectory'],
    title: 'Select Project Directory',
  });
  return result.canceled ? null : result.filePaths[0];
});

// ── App lifecycle ─────────────────────────────────────────────────────
app.whenReady().then(async () => {
  // 1. Spawn Rust backend
  spawnBackend();

  // 2. Wait for backend :3111
  try {
    await waitForPort(3111);
    console.log('[electron] backend ready on :3111');
  } catch (e) {
    console.warn('[electron] backend not ready, loading anyway:', e.message);
  }

  // 3. Create UI
  createWindow();
  createTray();
});

app.on('window-all-closed', (e) => {
  e.preventDefault(); // Keep running in tray
});

app.on('before-quit', () => {
  if (backendProcess) {
    backendProcess.kill();
    backendProcess = null;
  }
});
