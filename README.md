# FigTime 🎨

Show what you're designing in Figma on your Discord profile, automatically.

While you work, your Discord status shows:

- **Working on [your file name]**
- **Figma Desktop** or **Figma Browser**
- How long you've been working

If you step away — no typing or mouse movement for 5 minutes, or Figma left in the background for 5 minutes — it switches to **Idle in [your file name]**, and back as soon as you return.

It's a tiny app that runs quietly in your system tray and clears your status when you close Figma.

## Download & install

**[⬇ Download the latest version](https://github.com/krutikkkkkkkkk/FigTime/releases/latest)**

| Your computer | File |
|---|---|
| Windows 10/11 | `FigTime-…-windows.exe` |
| Mac (Apple chip or Intel) | `FigTime-…-mac.dmg` |

> **Upgrading from FigTime 1.x?** Uninstall the old version first (Windows → Settings → Apps → Figtime → Uninstall), so the two don't both update your status.

### Windows

1. Download `FigTime-…-windows.exe`. There's no installer — the file *is* the app.
2. Move it somewhere permanent, e.g. a `FigTime` folder in Documents. (It remembers where it is to start at login.)
3. Double-click it. Windows may show **"Windows protected your PC"** because FigTime isn't code-signed yet. Click **More info → Run anyway**.
4. FigTime appears in your system tray (bottom-right; you may need to click the **^** arrow).

### Mac

1. Open the `.dmg` and drag **FigTime** into **Applications**.
2. Open FigTime from Applications. The first time, macOS will say it can't be opened because it isn't from an identified developer.
3. Go to **System Settings → Privacy & Security**, scroll down, and click **Open Anyway** next to the FigTime message. Confirm with your password.
4. When asked, allow **Screen Recording** (System Settings → Privacy & Security → Screen Recording → turn on FigTime), then quit and reopen FigTime. FigTime only uses this to read window *titles*, which is how it gets your Figma file name. Without it, your status shows "Working on Figma" instead of the file name.
5. FigTime appears as an icon in your menu bar (top-right). It has no Dock icon.

### Make sure Discord shows it

- Use the **Discord desktop app** (not Discord in a browser) and keep it running.
- In Discord, go to **User Settings → Activity Privacy** and turn on **Share your detected activities with others**.

That's it — open Figma and your status updates within about 10 seconds.

## Using FigTime

Click the FigTime icon in the tray / menu bar to see what it's currently showing on Discord.

- **It starts automatically** when you log in. Turn this off with **Start at login** in the tray menu.
- **Order doesn't matter.** Start Discord before or after FigTime; it connects on its own and reconnects if Discord restarts.
- **Your status stays up** while Figma is open, even when you switch to other apps. After 5 minutes away it shows as **Idle**; the timer keeps running. It clears when you close Figma.
- **To quit:** tray icon → **Quit FigTime**.
- **To uninstall:** untick **Start at login**, quit FigTime, then delete the app (Windows: the `.exe`; Mac: FigTime in Applications).

### Supported browsers

Figma in Chrome, Edge, Firefox, Opera, Brave and Vivaldi works, plus Safari and Arc on Mac. Your Figma tab needs to be the visible tab in its browser window; keeping Figma in its own window works best.

## Troubleshooting

**My status doesn't show up**
- Check Discord's **Activity Privacy** setting (see above).
- Make sure you're using the Discord desktop app, not the website.
- Click the FigTime tray icon: the first line says what it's doing (e.g. "Waiting for Discord…", "Figma isn't open").
- Click into your Figma window once so FigTime picks it up.

**It says "Working on Figma" instead of my file name (Mac)**
- Turn on **Screen Recording** for FigTime in System Settings → Privacy & Security, then quit and reopen FigTime.

**Mac says FigTime "is damaged and can't be opened"**
- Open Terminal and run `xattr -cr /Applications/FigTime.app`, then open it again.

**Status disappears when I switch browser tabs**
- FigTime can only see the visible tab of each browser window. Move Figma into its own browser window, or use the Figma desktop app.

---

## For developers

FigTime is written in Rust. Install Rust from [rustup.rs](https://rustup.rs), then:

```bash
git clone https://github.com/krutikkkkkkkkk/FigTime.git
cd FigTime
cargo run              # debug build, with a console for logs
cargo test             # title parsing + idle logic tests
cargo build --release  # optimised for size -> target/release/figtime(.exe)
```

On Windows, use the default MSVC toolchain (needs Visual Studio Build Tools with "Desktop development with C++"). On a Mac, `bash scripts/bundle-macos.sh` builds the universal `.app` and `.dmg` after building both targets (see the script).

Options (environment variables):

| Variable | Purpose |
|---|---|
| `CLIENT_ID` | Use your own Discord application. Create one in the [Discord Developer Portal](https://discord.com/developers/applications) and upload a Figma logo under **Rich Presence → Art Assets** with the key `figma`. |
| `IDLE_AFTER_SECONDS` | Shorten the idle delay for testing (default 300). |

Debug builds and builds run from `target/` never register themselves to start at login, and can run alongside an installed FigTime.

### Publish a release

Releases are built by GitHub Actions ([.github/workflows/main.yml](.github/workflows/main.yml)):

1. Bump `version` in [Cargo.toml](Cargo.toml) (e.g. `2.1.0`) and commit.
2. Tag and push: `git tag v2.1.0 && git push --follow-tags`.
3. GitHub Actions runs the tests, builds the Windows `.exe` and the universal Mac `.dmg`, and publishes them as a public release. The tag must match the `Cargo.toml` version or the release fails.

Every push to `main` also builds both, available as artifacts on the workflow run (not published).

### How it works

Every 10 seconds FigTime looks for a Figma window — the Figma desktop app, or a browser tab whose title ends in "– Figma" — first the focused window, then all open windows, then the Figma process. It reads the window title to get the file name and sends it to Discord over Discord's local IPC connection (Rich Presence). It shows **Idle** when there's been no keyboard/mouse input, or Figma hasn't been focused, for 5 minutes, and clears the status once Figma is closed.

| Code | Purpose |
|---|---|
| [src/main.rs](src/main.rs) | Tray menu and the background check loop |
| [src/figma.rs](src/figma.rs) | Figma detection, title parsing, idle/timer logic |
| [src/discord.rs](src/discord.rs) | Minimal Discord Rich Presence client |
| [src/platform/](src/platform/) | Windows / macOS window titles, idle time, start at login |
| [build.rs](build.rs) | Turns `assets/icon.png` into the tray and `.exe` icons at build time |

### Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes and run `cargo test`
4. Open a pull request

## License

[MIT License](LICENSE). Feel free to use, modify, and distribute this code under its terms.

## Support

Found a bug? [Open an issue](https://github.com/krutikkkkkkkkk/FigTime/issues). Follow me on X for updates: [@krutikkkkkkkkk](https://x.com/krutikkkkkkkkk)
