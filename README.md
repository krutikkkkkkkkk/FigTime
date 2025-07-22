# FigTime 🎨

A Discord Rich Presence application that automatically displays your Figma activity status on Discord.

## Features

- **Automatic Detection**: Monitors both Figma Desktop app and browser tabs
- **Smart Activity Tracking**: Shows current project name and platform (Desktop/Browser)
- **Intelligent Clearing**: Automatically clears presence when Figma is closed or inactive
- **Cross-Platform Support**: Works with Figma Desktop and web browsers
- **Real-time Updates**: Updates every 5 seconds for accurate status

## Prerequisites

- Node.js (v14 or higher)
- Discord Desktop application
- Figma Desktop app or web browser with Figma

## Installation

1. Clone or download this repository
2. Install dependencies:
   ```bash
   npm install
   ```
3. Create a `.env` file in the project root:
   ```bash
   CLIENT_ID=your_discord_application_id_here
   ```

## Setup

### Discord Application Setup

1. Go to [Discord Developer Portal](https://discord.com/developers/applications)
2. Create a new application and give it a name (e.g., "FigTime")
3. Go to the "General Information" tab and copy the **Application ID**
4. Create a `.env` file in your project directory and add:
   ```
   CLIENT_ID=your_application_id_here
   ```
5. Go to "Rich Presence" → "Art Assets" and upload a Figma logo with the key name "figma"

### Environment Variables

Create a `.env` file with the following variables:

| Variable | Description | Required |
|----------|-------------|----------|
| `CLIENT_ID` | Your Discord Application ID | Yes |

## Usage

1. Make sure Discord is running on your computer
2. Ensure your `.env` file is properly configured
3. Start FigTime:
   ```bash
   npm start
   ```
   or
   ```bash
   node index.js
   ```
4. Open Figma (either desktop app or in your browser)
5. Your Discord status will automatically update to show your Figma activity!

## How It Works

FigTime monitors your system for Figma activity in two ways:

1. **Active Window Detection**: Checks if the currently focused window is Figma
2. **Process Detection**: Scans running processes for Figma instances

When Figma activity is detected, it extracts the project name from the window title and displays:
- **Details**: "Working on [Project Name]"
- **State**: "Figma Desktop" or "Figma Browser"
- **Duration**: How long you've been working on the current project

## Browser Support

FigTime works with the following browsers:
- Google Chrome
- Mozilla Firefox
- Microsoft Edge
- Safari
- Opera
- Brave

## Configuration

The application uses environment variables for configuration. Create a `.env` file in the project root:

```env
CLIENT_ID=your_discord_application_id_here
```

Additional configurable parameters in `index.js`:

- `FIGMA_TIMEOUT`: Time (in milliseconds) before clearing presence when no Figma activity is detected (default: 30 seconds)
- Update interval: How often to check for activity (default: 5 seconds)

## Discord Application Setup

This project requires you to create your own Discord application:

1. Go to [Discord Developer Portal](https://discord.com/developers/applications)
2. Create a new application and give it a name (e.g., "FigTime")
3. Copy the Application ID from the "General Information" tab
4. Add the Application ID to your `.env` file as `CLIENT_ID`
5. Go to "Rich Presence" → "Art Assets" and upload a Figma icon with the key name "figma"

The application will use your Discord Application ID from the environment variables to connect to Discord.

## Troubleshooting

### "Discord not detected" error
- Make sure Discord Desktop is running (not just the web version)
- Try restarting Discord
- Verify your `.env` file contains a valid `CLIENT_ID`

### Activity not showing
- Ensure Figma is the active/focused window
- Check that the project has a proper title in Figma
- Verify that Discord's activity status is enabled in Discord settings
- Make sure your Discord application has the "figma" asset uploaded

### Environment variable errors
- Ensure your `.env` file is in the project root directory
- Check that `CLIENT_ID` is properly set in the `.env` file
- Restart the application after making changes to `.env`

### Permission errors on Windows
- Try running as administrator if you encounter PowerShell execution policy issues

## Project Structure

```
FigTime/
├── index.js          # Main application file
├── package.json      # Node.js dependencies and scripts
├── package-lock.json # Exact dependency versions
├── .env              # Environment variables (create this)
├── .gitignore        # Git ignore rules
└── README.md         # This file
```

## Dependencies

- **discord-rpc**: Enables Discord Rich Presence integration
- **active-win**: Detects the currently active window
- **dotenv**: Loads environment variables from .env file

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Test thoroughly
5. Submit a pull request

## License

This project is licensed under the  [MIT License](LICENSE). Feel free to use, modify, and distribute this code as per the terms of the license. 

## Support

If you encounter any issues or have questions:
1. Check the troubleshooting section above
2. Make sure all prerequisites are installed
3. Verify that Discord and Figma are running properly

---

**Note**: This application requires Discord Desktop to be running and may need appropriate permissions to monitor system processes on your operating system.

## Follow me on X (formerly Twitter) for updates and support: [@krutikkkkkkkkk](https://x.com/krutikkkkkkkkk)
