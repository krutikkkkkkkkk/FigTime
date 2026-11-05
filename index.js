import dotenv from "dotenv";
import rpc from "discord-rpc";
import { activeWindow as activeWin, openWindows } from "get-windows";
import { app, Tray, Menu, nativeImage } from 'electron';
import path from 'path';
import { fileURLToPath } from 'url';
import AutoLaunch from 'auto-launch';

// Get __dirname equivalent in ES modules
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

dotenv.config({ quiet: true });

// CLIENT_ID from .env overrides the bundled Discord application ID
const clientId = process.env.CLIENT_ID || "1396898295594483753";
rpc.register(clientId);

let client = null;
let connected = false;
let presenceTimer = null;
let reconnectTimer = null;
const RECONNECT_DELAY = 15000; // Retry Discord connection every 15 seconds
let tray = null;

function createTray() {
  let icon = nativeImage.createFromPath(path.join(__dirname, 'assets', 'icon.png'));
  if (process.platform === 'darwin') {
    // Menu bar icons are ~18pt tall; the source icon is 512px
    icon = icon.resize({ width: 18, height: 18 });
  }
  tray = new Tray(icon);
  const menu = Menu.buildFromTemplate([
    { label: 'Quit Figtime', click: () => app.quit() },
  ]);
  tray.setToolTip('Figtime is running');
  tray.setContextMenu(menu);
}

// Global variables for presence tracking
let startTimestamp = new Date();
let lastActivity = null;
let hasEverSetActivity = false;

app.whenReady().then(() => {
  // Only one FigTime at a time (e.g. relaunching or installing over a running copy)
  if (!app.requestSingleInstanceLock()) {
    app.quit();
    return;
  }

  // Tray-only app: no Dock icon on macOS
  if (process.platform === 'darwin') app.dock?.hide();

  createTray();

  const autoLauncher = new AutoLaunch({ name: 'Figtime' });
  autoLauncher.enable().catch(console.error);

  connectToDiscord();
});

// Connect to the local Discord client, retrying until Discord is running
function connectToDiscord() {
  clearTimeout(reconnectTimer);
  const thisClient = new rpc.Client({ transport: "ipc" });
  client = thisClient;

  thisClient.on("ready", () => {
    if (client !== thisClient) return;
    connected = true;
    lastActivity = null; // Force the current activity to be re-sent
    console.log("✅ FigTime connected to Discord");
    console.log(`[INFO] Running on ${process.platform === 'darwin' ? 'macOS' : process.platform === 'win32' ? 'Windows' : 'Linux'}`);
    updatePresence();
    if (!presenceTimer) presenceTimer = setInterval(updatePresence, 10000); // Check every 10 seconds
  });

  // Events from a client we've already replaced are ignored
  thisClient.on("disconnected", () => {
    if (client !== thisClient) return;
    console.log("[⚠] Disconnected from Discord - will retry");
    scheduleReconnect();
  });

  thisClient.login({ clientId }).catch(() => {
    if (client !== thisClient) return;
    console.log(`[⚠] Discord not available - retrying in ${RECONNECT_DELAY / 1000}s`);
    scheduleReconnect();
  });
}

function scheduleReconnect() {
  connected = false;
  clearTimeout(reconnectTimer);
  const oldClient = client;
  client = null;
  oldClient?.destroy().catch(() => {});
  reconnectTimer = setTimeout(connectToDiscord, RECONNECT_DELAY);
}

// Check if Figma processes are running
async function isFigmaProcessRunning() {
  try {
    const { execSync } = await import('child_process');
    let command;
    
    if (process.platform === 'win32') {
      // Windows - use PowerShell
      command = 'powershell "Get-Process | Where-Object {$_.ProcessName -like \'*figma*\'} | Select-Object ProcessName"';
    } else if (process.platform === 'darwin') {
      // macOS - use ps and grep
      command = 'ps aux | grep -i figma | grep -v grep';
    } else {
      // Linux - use ps and grep
      command = 'ps aux | grep -i figma | grep -v grep';
    }
    
    const result = execSync(command, { encoding: 'utf8', timeout: 5000 });
    return result.includes('Figma') || result.includes('figma');
  } catch (error) {
    return false;
  }
}

// Check if any browser has Figma tabs
// (looks at every open window, not just the focused one)
async function hasFigmaBrowserTab() {
  try {
    const windows = await openWindows();
    return windows.some(isFigmaBrowser);
  } catch (error) {
    return false;
  }
}

// Browser app names, matched against the window owner's name
const BROWSER_NAMES =
  process.platform === 'darwin'
    ? ['chrome', 'firefox', 'safari', 'edge', 'opera', 'brave', 'arc']
    : process.platform === 'win32'
      ? ['chrome', 'firefox', 'edge', 'msedge', 'opera', 'brave']
      : ['chrome', 'firefox', 'chromium', 'opera', 'brave', 'epiphany'];

// Figma titles end in " – Figma" / " - Figma", optionally followed by
// the browser/profile (e.g. "My file – Figma - Google Chrome",
// "My file – Figma and 2 more pages - Personal - Microsoft Edge").
// Requiring this suffix avoids matching pages that merely mention Figma.
const FIGMA_TITLE_SUFFIX = /\s+[–—-]\s+Figma(\s+and\s+\d+\s+more\s+pages?)?(\s+[–—-]\s+.*)?\s*$/i;

function isBrowserWindow(window) {
  const appName = (window?.owner?.name || '').toLowerCase();
  return BROWSER_NAMES.some(browser => appName.includes(browser));
}

// Check if window is Figma desktop app
function isFigmaDesktop(window) {
  if (!window?.owner?.name) return false;
  return window.owner.name.toLowerCase().includes('figma');
}

// Check if window is browser with Figma tab
function isFigmaBrowser(window) {
  if (!isBrowserWindow(window)) return false;

  // macOS can also give us the tab URL
  let hasFigmaUrl = false;
  try {
    const host = new URL(window.url || '').hostname;
    hasFigmaUrl = host === 'figma.com' || host.endsWith('.figma.com');
  } catch {}

  return hasFigmaUrl || FIGMA_TITLE_SUFFIX.test(window.title || '');
}

// Discord rejects "details" longer than 128 characters
const MAX_DETAILS_LENGTH = 128;

// Extract project name from window title
function extractProjectName(title) {
  const cleanTitle = (title || '').replace(FIGMA_TITLE_SUFFIX, '').trim();
  const name = cleanTitle || 'Figma Project';

  const maxNameLength = MAX_DETAILS_LENGTH - 'Working on '.length;
  return name.length > maxNameLength ? name.slice(0, maxNameLength - 1) + '…' : name;
}

// Main presence update function
async function updatePresence() {
  if (!connected) return; // Waiting for Discord; reconnect logic handles it
  try {
    const window = await activeWin();

    // Debug logging for browser detection
    if (window?.owner?.name) {
      const appName = window.owner.name.toLowerCase();
      const title = (window.title || '').toLowerCase();
      
      // Platform-specific browser detection for debug logging
      let browserList;
      if (process.platform === 'darwin') {
        browserList = ['chrome', 'firefox', 'safari', 'edge', 'opera', 'brave', 'arc', 'google chrome', 'mozilla firefox'];
      } else if (process.platform === 'win32') {
        browserList = ['chrome', 'firefox', 'edge', 'msedge', 'safari', 'opera', 'brave', 'iexplore'];
      } else {
        browserList = ['chrome', 'firefox', 'chromium', 'safari', 'opera', 'brave', 'epiphany'];
      }
      
      const isBrowserApp = browserList.some(browser => appName.includes(browser));
      
      if (isBrowserApp && title.includes('figma')) {
        console.log(`[DEBUG] Browser window detected: ${window.owner.name}`);
        console.log(`[DEBUG] Title: ${window.title}`);
        console.log(`[DEBUG] URL: ${window.url || 'Not available'}`);
      }
    }
    
    // Check for Figma usage
    const isDesktop = isFigmaDesktop(window);
    const isBrowser = isFigmaBrowser(window);
    
    // Debug output for detection results
    if (window?.owner?.name && (isDesktop || isBrowser)) {
      console.log(`[DEBUG] Figma detected - Desktop: ${isDesktop}, Browser: ${isBrowser}`);
    }
    
    if (isDesktop || isBrowser) {
      // We found active Figma - update presence
      const projectName = extractProjectName(window.title);
      const platform = isDesktop ? 'Desktop' : 'Browser';
      const activityKey = `${projectName}-${platform}`;

      // Timer starts once per session and only restarts after Figma is closed
      if (!hasEverSetActivity) {
        startTimestamp = new Date();
      }
      
      const newActivity = {
        details: `Working on ${projectName}`,
        state: `Figma ${platform}`,
        startTimestamp,
        largeImageKey: "figma",
        largeImageText: `Figma ${platform}`,
        instance: false,
      };

      // Only update if activity changed
      if (lastActivity !== activityKey) {
        client.setActivity(newActivity).catch(console.error);
        console.log(`[✔] Updated: ${projectName} (${platform})`);
        lastActivity = activityKey;
        hasEverSetActivity = true;
      }
    } else {
      // No active Figma window - check if Figma is still running
      const figmaRunning = await isFigmaProcessRunning();
      const browserFigma = await hasFigmaBrowserTab();
      
      if (!figmaRunning && !browserFigma) {
        // No Figma found anywhere - clear immediately
        if (hasEverSetActivity) {
          clearPresence();
          console.log('[⚠] Figma closed - presence cleared');
        }
      }
      // Figma still open but not focused - keep the current presence and timer
    }
    
  } catch (error) {
    console.error("Error updating presence:", error);
  }
}

// Clear presence manually (call this when you want to clear)
function clearPresence() {
  if (hasEverSetActivity) {
    if (connected) client.clearActivity().catch(() => {});
    console.log('[✔] Presence cleared');
    lastActivity = null;
    hasEverSetActivity = false;
  }
}


// Handle app events
app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

app.on('activate', () => {
  if (!tray || tray.isDestroyed()) {
    createTray();
  }
});

// Handle graceful shutdown
app.on('before-quit', () => {
  console.log('FigTime is quitting');
  clearPresence();
});

process.on('SIGINT', () => {
  console.log('\n[⚠] Shutting down FigTime...');
  clearPresence();
  process.exit(0);
});

