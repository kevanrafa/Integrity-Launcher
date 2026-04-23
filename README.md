# Integrity Launcher

Integrity Launcher is a fork of Pandora Launcher by Moulberry,
with additional modifications and features by kevanrafa10 (Kriss).

Work in progress

## Features
- Instance management
- Cross-instance file syncing (options, saves, etc.) (https://youtu.be/wb5EY2VsMKg)
- Mod deduplication when installed through launcher (using hard links)
- Secure account credential management using platform keyrings
- Custom game output window
- Mod browser using Modrinth's API
- Automatic redaction of sensitive information (i.e. access tokens) in logs
- Unique approach to modpack management (https://youtu.be/cdRVqd7b2BQ)
- Offline Mode (Developer Mode) - login without Microsoft account
- Discord Rich Presence support
- Developer Mode settings

## Fork Features (Integrity Launcher)
- Rebranded from Pandora Launcher
- Offline Mode for development purposes (username-based login without Microsoft authentication)
- Discord RPC integration for Discord Presence
- Developer Mode toggle for debug features and advanced settings

## Implementation Notes

### Discord RPC

- App ID: `1473107584847188119`
- Presence is handled in `crates/frontend/src/discord_rpc.rs`
- Line 1 shows the active instance name when Minecraft is running
- Line 2 rotates through random flavor text
- The setting is controlled from the Discord tab in launcher settings
- To extend it, add more status lines or richer assets in the RPC worker

### Java Management

- Java preference and compatibility handling lives in the backend launch pipeline
- Legacy versions are forced toward Java 8 compatibility rules
- Modern versions can use newer managed runtimes when available
- To extend it, continue from `crates/backend/src/launch/mod.rs` and backend config handling

### Modpack System

- Integrity Launcher keeps Pandora's content-based install flow and deduplication behavior
- Mod and modpack install/update flows are split between frontend selection UI and backend installers
- To extend it, look at `crates/backend/src/install_content.rs` plus the Modrinth/Curseforge pages in `crates/frontend/src/pages`

### Boot Sequence

- The startup animation lives in `crates/frontend/src/boot_sequence.rs`
- It runs for about 2-3 seconds, supports click-to-skip, and has a rare root-mode variation
- To extend it, add lines to the line pools or adjust timing constants near the top of the file

### Windows Publisher Note

- File metadata such as `CompanyName` and `ProductName` can be embedded at build time
- Windows SmartScreen and the security warning publisher field will still show `Unknown Publisher` until the executable is signed with a real Authenticode code-signing certificate

## FAQ

### Where can I suggest a feature/report a bug?

Please use GitHub issues.

### Why should I use Integrity Launcher over other launchers?

This fork adds Offline Mode and developer-oriented features useful for testing Minecraft mods and development.

### Will Integrity Launcher be monetized?

Unlikely. This is a fork of Pandora Launcher, maintaining similar principles:

- It is wrong for launchers to be monetized without distributing revenue back to mod creators that give the launcher value in the first place.
- Dealing with monetization takes a lot of (ongoing) work.
- Personal dislike for advertisements.

## Credits

- Original Pandora Launcher by Moulberry
- Fork by Kriss (kevinrafa10)
- Uses GPUi framework from Zed Industries
