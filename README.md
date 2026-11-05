# FigTime 🎨

Show what you're designing in Figma on your Discord profile, automatically.

While you work, your Discord status shows:

- **Working on [your file name]**
- **Figma Desktop** or **Figma Browser**
- How long you've been working

It runs quietly in your system tray and clears your status when you close Figma.

## Download & install

**[⬇ Download the latest version](https://github.com/krutikkkkkkkkk/FigTime/releases/latest)**

Pick the file for your computer:

| Your computer | File |
|---|---|
| Windows 10/11 | `FigTime-…-windows-setup.exe` |
| Mac with Apple chip (M1/M2/M3/M4) | `FigTime-…-mac-arm64.dmg` |
| Mac with Intel chip | `FigTime-…-mac-x64.dmg` |

Not sure which Mac you have? Apple menu → **About This Mac** → look for "Chip" (Apple) or "Processor" (Intel).

### Windows

1. Run `FigTime-…-windows-setup.exe`.
2. Windows may show **"Windows protected your PC"**. This happens because FigTime isn't code-signed yet. Click **More info → Run anyway**.
3. Finish the installer. FigTime starts and appears in your system tray (bottom-right, you may need to click the **^** arrow).

### Mac

1. Open the `.dmg` and drag **Figtime** into **Applications**.
2. Open Figtime from Applications. The first time, macOS will say it can't be opened because FigTime isn't from an identified developer.
3. Go to **System Settings → Privacy & Security**, scroll down, and click **Open Anyway** next to the FigTime message. Confirm with your password.
4. When asked, allow **Screen Recording** (System Settings → Privacy & Security → Screen Recording → turn on Figtime). FigTime only uses this to read window *titles*, which is how it gets your Figma file name. Without it, your status shows "Figma Project" instead of the file name.
5. FigTime appears as an icon in your menu bar (top-right). It has no Dock icon.

### Make sure Discord shows it

- Use the **Discord desktop app** (not Discord in a browser) and keep it running.
- In Discord, go to **User Settings → Activity Privacy** and turn on **Share your detected activities with others**.

That's it — open Figma and your status updates within about 10 seconds.

## Using FigTime

- **It starts automatically** when you log in to your computer.
- **Order doesn't matter.** Start Discord before or after FigTime; it connects on its own and reconnects if Discord restarts.
- **Your status stays up** while Figma is open, even when you switch to other apps. It clears when you close Figma.
- **To quit:** right-click (Windows) or click (Mac) the FigTime tray icon → **Quit Figtime**.
- **To stop it starting at login:** Windows → Task Manager → Startup apps → disable Figtime. Mac → System Settings → General → Login Items → remove Figtime.
- **To uninstall:** Windows → Settings → Apps → Figtime → Uninstall. Mac → drag Figtime from Applications to the Trash.

### Supported browsers

Figma in Chrome, Edge, Firefox, Opera or Brave works, plus Safari and Arc on Mac. Your Figma tab needs to be the visible tab in its browser window; keeping Figma in its own window works best.

## Troubleshooting

**My status doesn't show up**
- Check Discord's **Activity Privacy** setting (see above).
- Make sure you're using the Discord desktop app, not the website.
- Check FigTime is running (look for its icon in the tray / menu bar).
- Click into your Figma window once so FigTime picks it up.

**It says "Figma Project" instead of my file name (Mac)**
- Turn on **Screen Recording** for Figtime in System Settings → Privacy & Security, then quit and reopen FigTime.

**Mac says FigTime "is damaged and can't be opened"**
- Open Terminal and run `xattr -cr /Applications/Figtime.app`, then open it again.

**Status disappears when I switch browser tabs**
- FigTime can only see the visible tab of each browser window. Move Figma into its own browser window, or use the Figma desktop app.

---

## For developers

### Run from source

Requires [Node.js](https://nodejs.org) 22 or newer.

```bash
git clone https://github.com/krutikkkkkkkkk/FigTime.git
cd FigTime
npm install
npm start
```

FigTime uses a built-in Discord application ID, so no setup is needed. To use your own Discord application instead, create one in the [Discord Developer Portal](https://discord.com/developers/applications), upload a Figma logo under **Rich Presence → Art Assets** with the key `figma`, and put its Application ID in a `.env` file:

```env
CLIENT_ID=your_application_id
```

> Running `npm start` from VS Code's terminal? If you see `does not provide an export named 'Menu'`, the terminal has `ELECTRON_RUN_AS_NODE=1` set. Unset it or use a separate terminal.

### Build installers locally

```bash
npm run dist:win   # Windows .exe installer (run on Windows)
npm run dist:mac   # macOS .dmg for Apple chip + Intel (run on a Mac)
```

Output goes to `dist/`.

### Publish a release

Releases are built by GitHub Actions ([.github/workflows/main.yml](.github/workflows/main.yml)):

1. Bump the version: `npm version 1.2.0` (updates `package.json` and creates the `v1.2.0` tag).
2. Push it: `git push --follow-tags`.
3. GitHub Actions builds the Windows and Mac installers and publishes them as a public release. The tag must match the `package.json` version or the build fails.

Every push to `main` also builds installers, available as artifacts on the workflow run (not published).

### How it works

Every 10 seconds FigTime checks the focused window. If it's the Figma desktop app or a browser showing Figma, it reads the window title to get the file name and sends it to Discord over its local connection (Discord Rich Presence). When Figma isn't focused, it checks whether Figma is still open and keeps your status; once Figma is closed, it clears it.

| Package | Purpose |
|---|---|
| [electron](https://www.electronjs.org) | Tray app and packaging |
| [discord-rpc](https://www.npmjs.com/package/discord-rpc) | Discord Rich Presence |
| [get-windows](https://www.npmjs.com/package/get-windows) | Reads the focused window |
| [auto-launch](https://www.npmjs.com/package/auto-launch) | Start at login |
| [dotenv](https://www.npmjs.com/package/dotenv) | Optional `.env` config |

### Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes and test them
4. Open a pull request

## License

[MIT License](LICENSE). Feel free to use, modify, and distribute this code under its terms.

## Support

Found a bug? [Open an issue](https://github.com/krutikkkkkkkkk/FigTime/issues). Follow me on X for updates: [@krutikkkkkkkkk](https://x.com/krutikkkkkkkkk)
