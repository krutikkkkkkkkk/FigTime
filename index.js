import dotenv from "dotenv";
dotenv.config();
import rpc from "discord-rpc";
import activeWin from "active-win";

const clientId = process.env.CLIENT_ID;
rpc.register(clientId);

const client = new rpc.Client({ transport: "ipc" });

let startTimestamp = new Date();
let lastActivity = null;
let hasEverSetActivity = false;
let lastFigmaCheck = 0; // Track when we last saw Figma
const FIGMA_TIMEOUT = 30000; // Clear after 30 seconds of no Figma detection

// Check if Figma processes are running
async function isFigmaProcessRunning() {
  try {
    const { execSync } = await import('child_process');
    // Check for Figma desktop processes
    const result = execSync('powershell "Get-Process | Where-Object {$_.ProcessName -like \'*figma*\'} | Select-Object ProcessName"', 
      { encoding: 'utf8', timeout: 5000 });
    return result.includes('Figma') || result.includes('figma');
  } catch (error) {
    return false;
  }
}

// Check if any browser has Figma tabs
async function hasFigmaBrowserTab() {
  try {
    const { execSync } = await import('child_process');
    // Check if any browser process has figma.com in its window title or URL
    const result = execSync('powershell "Get-Process | Where-Object {$_.MainWindowTitle -like \'*figma*\'} | Select-Object ProcessName, MainWindowTitle"', 
      { encoding: 'utf8', timeout: 5000 });
    
    // Check if any browser process mentions figma
    const browserProcesses = ['chrome', 'firefox', 'edge', 'msedge', 'safari', 'opera', 'brave'];
    const lines = result.split('\n');
    
    for (const line of lines) {
      const lowerLine = line.toLowerCase();
      if (browserProcesses.some(browser => lowerLine.includes(browser)) && 
          lowerLine.includes('figma')) {
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
  
  const isBrowser = ['chrome', 'firefox', 'edge', 'msedge', 'safari', 'opera', 'brave']
    .some(browser => appName.includes(browser));
  
  // Check both URL and title for figma.com
  return isBrowser && (url.includes('figma.com') || title.includes('figma'));
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
    
    // Check for Figma usage
    const isDesktop = isFigmaDesktop(window);
    const isBrowser = isFigmaBrowser(window);
    
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
  updatePresence();
  setInterval(updatePresence, 5000); // Check every 5 seconds
});

// Handle graceful shutdown
process.on('SIGINT', () => {
  console.log('\n[⚠] Shutting down FigTime...');
  clearPresence();
  process.exit(0);
});

client.login({ clientId }).catch(console.error);
