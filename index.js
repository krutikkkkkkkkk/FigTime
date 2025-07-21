import dotenv from "dotenv";
import rpc from "discord-rpc";
import activeWin from "active-win";
import { app, Tray, Menu } from 'electron';
import path from 'path';
import { fileURLToPath } from 'url';
import AutoLaunch from 'auto-launch';

// Get __dirname equivalent in ES modules
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

dotenv.config();

const clientId = process.env.CLIENT_ID;
rpc.register(clientId);

const client = new rpc.Client({ transport: "ipc" });
let tray = null;

// Global variables for presence tracking
let startTimestamp = new Date();
let lastActivity = null;
let hasEverSetActivity = false;
let lastFigmaCheck = 0; // Track when we last saw Figma
const FIGMA_TIMEOUT = 30000; // Clear after 30 seconds of no Figma detection

app.whenReady().then(() => {
  tray = new Tray(path.join(__dirname, 'assets', 'icon.png'));
  const menu = Menu.buildFromTemplate([
    { label: 'Quit Figtime', click: () => app.quit() },
  ]);
  tray.setToolTip('Figtime is running');
  tray.setContextMenu(menu);

  const autoLauncher = new AutoLaunch({ name: 'Figtime' });
  autoLauncher.enable();

  // Start Discord RPC connection
  client.login({ clientId }).catch(console.error);
});

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
async function hasFigmaBrowserTab() {
  try {
    const { execSync } = await import('child_process');
    let command;
    
    if (process.platform === 'win32') {
      // Windows - use PowerShell to check window titles
      command = 'powershell "Get-Process | Where-Object {$_.MainWindowTitle -ne \'\' -and $_.MainWindowTitle -like \'*figma*\'} | Select-Object ProcessName, MainWindowTitle"';
    } else if (process.platform === 'darwin') {
      // macOS - check running processes with figma
      command = 'ps aux | grep -i figma | grep -v grep';
    } else {
      // Linux - use wmctrl or xdotool if available
      command = 'wmctrl -l | grep -i figma || xdotool search --name figma 2>/dev/null || true';
    }
    
    const result = execSync(command, { encoding: 'utf8', timeout: 5000 });
    
    // Platform-specific browser process names
    let browserProcesses;
    if (process.platform === 'darwin') {
      browserProcesses = ['chrome', 'firefox', 'safari', 'edge', 'opera', 'brave', 'arc', 'google chrome', 'mozilla firefox'];
    } else if (process.platform === 'win32') {
      browserProcesses = ['chrome', 'firefox', 'edge', 'msedge', 'safari', 'opera', 'brave', 'iexplore'];
    } else {
      browserProcesses = ['chrome', 'firefox', 'chromium', 'safari', 'opera', 'brave', 'epiphany'];
    }
    
    const lines = result.split('\n');
    
    for (const line of lines) {
      const lowerLine = line.toLowerCase();
      // Check if line contains both a browser name and figma
      const hasBrowser = browserProcesses.some(browser => lowerLine.includes(browser));
      const hasFigma = lowerLine.includes('figma');
      
      if (hasBrowser && hasFigma) {
        return true;
      }
    }
    
    return false;
  } catch (error) {
    // Fallback to checking current window
    try {
      const window = await activeWin();
      return isFigmaBrowser(window);
    } catch (fallbackError) {
      return false;
    }
  }
}

// Check if window is Figma desktop app
function isFigmaDesktop(window) {
  if (!window?.owner?.name) return false;
  return window.owner.name.toLowerCase().includes('figma');
}

// Check if window is browser with Figma tab
function isFigmaBrowser(window) {
  if (!window?.owner?.name) return false;

  const appName = window.owner.name.toLowerCase();
  const url = (window.url || '').toLowerCase();
  const title = (window.title || '').toLowerCase();

  // Browser names vary by platform
  let browserNames;
  if (process.platform === 'darwin') {
    // macOS browser names
    browserNames = ['chrome', 'firefox', 'safari', 'edge', 'opera', 'brave', 'arc', 'google chrome', 'mozilla firefox'];
  } else if (process.platform === 'win32') {
    // Windows browser names
    browserNames = ['chrome', 'firefox', 'edge', 'msedge', 'safari', 'opera', 'brave', 'iexplore'];
  } else {
    // Linux browser names
    browserNames = ['chrome', 'firefox', 'chromium', 'safari', 'opera', 'brave', 'epiphany'];
  }

  const isBrowser = browserNames.some(browser => appName.includes(browser));

  // Check URL or title for figma.com
  const hasFigmaUrl = url.includes('figma.com');
  const hasFigmaTitle = title.includes('figma') || title.includes('- figma');
  
  return isBrowser && (hasFigmaUrl || hasFigmaTitle);
}

// Extract project name from window title
function extractProjectName(title) {
  if (!title) return 'Unknown Project';
  
  // Remove common Figma suffixes
  let cleanTitle = title
    .replace(/ – Figma$/i, '')
    .replace(/ - Figma$/i, '')
    .replace(/ - Figma - .+$/i, '') // Remove "- Figma - Browser Name"
    .trim();
  
  return cleanTitle || 'Figma Project';
}

// Main presence update function
async function updatePresence() {
  try {
    const window = await activeWin();
    const currentTime = Date.now();
    
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
      lastFigmaCheck = currentTime;
      const projectName = extractProjectName(window.title);
      const platform = isDesktop ? 'Desktop' : 'Browser';
      
      // Check if this is a new activity (only platform matters for timestamp reset)
      const activityKey = `${projectName}-${platform}`;
      const platformKey = platform;
      const isPlatformChange = lastActivity && !lastActivity.includes(platform);
      
      // Reset timestamp only for platform changes (Desktop <-> Browser)
      if (isPlatformChange || !hasEverSetActivity) {
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
        client.setActivity(newActivity);
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
      } else if (currentTime - lastFigmaCheck > FIGMA_TIMEOUT) {
        // Haven't seen active Figma for too long - probably minimized/background
        if (hasEverSetActivity) {
          clearPresence();
          console.log('[⚠] No Figma activity detected for 30s - presence cleared');
        }
      }
    }
    
  } catch (error) {
    console.error("Error updating presence:", error);
  }
}

// Clear presence manually (call this when you want to clear)
function clearPresence() {
  if (hasEverSetActivity) {
    client.clearActivity();
    console.log('[✔] Presence cleared');
    lastActivity = null;
    hasEverSetActivity = false;
  }
}

// Start the application
client.on("ready", () => {
  console.log("✅ FigTime connected to Discord");
  console.log(`[INFO] Running on ${process.platform === 'darwin' ? 'macOS' : process.platform === 'win32' ? 'Windows' : 'Linux'}`);
  updatePresence();
  setInterval(updatePresence, 10000); // Check every 10 seconds
});

client.login({ clientId }).catch(console.error);

// Handle app events
app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

app.on('activate', () => {
  if (Tray.getAllTrays().length === 0) {
    tray = new Tray(path.join(__dirname, 'assets', 'icon.png'));
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

