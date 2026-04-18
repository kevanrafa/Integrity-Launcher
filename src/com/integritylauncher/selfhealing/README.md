# Integrity Launcher Self-Healing System - Documentation

## Overview

The Self-Healing System is a modular component designed to ensure the Integrity Launcher can always launch Minecraft, even if files are missing or corrupted. It provides automatic validation, repair, Java runtime management, and comprehensive logging.

## Components

### 1. **Validator Interface**
Base interface for all validation and repair components.

```java
public interface Validator {
    boolean validate() throws Exception;
    void repair() throws Exception;
    String getName();
}
```

### 2. **LibraryManager**
Validates and repairs Minecraft libraries (LWJGL, JARs, assets).

**Features:**
- Validates all libraries and assets
- Detects empty/corrupted files
- Downloads missing libraries with SHA-1 verification
- Provides library statistics

**Usage:**
```java
LibraryManager libraryManager = new LibraryManager(
    librariesDir, assetsDir, versionJarsDir, 3, 10000);

if (!libraryManager.validate()) {
    libraryManager.repair();
}
```

### 3. **NativesManager**
Handles extraction and validation of native libraries (.dll, .so, .dylib).

**Features:**
- Extracts natives from JAR files
- Platform-specific native file handling
- Validates extracted natives
- Retry extraction on failure
- Clean repair functionality

**Usage:**
```java
NativesManager nativesManager = new NativesManager(
    nativesDir, librariesDir, tempDir, 3);

if (!nativesManager.validate()) {
    nativesManager.repair();
}
```

### 4. **JavaManager**
Detects and manages Java runtime installations.

**Features:**
- Detects installed Java versions
- Determines required Java version for Minecraft version
- Auto-downloads from Adoptium if missing
- Platform-specific Java handling
- Java version validation

**Java Version Mapping:**
- Java 8: Minecraft < 1.17
- Java 17: Minecraft 1.17-1.19
- Java 21+: Minecraft 1.20+

**Usage:**
```java
JavaManager javaManager = new JavaManager(runtimeDir, 10000, 3);

String javaPath = javaManager.selectJavaForMinecraft("1.20.1");
Map<Integer, String> installedJava = javaManager.detectInstalledJava();
```

### 5. **LogManager**
Captures, stores, and uploads game logs.

**Features:**
- Captures stdout/stderr from Minecraft process
- Saves logs to `/logs/latest.log`
- Uploads to mclo.gs via HTTP POST
- Returns shareable log links
- Log rotation support
- Asynchronous log upload

**Usage:**
```java
LogManager logManager = new LogManager(logsDir, 10 * 1024 * 1024);

logManager.captureProcessLogs(minecraftProcess);
logManager.logInfo("Game launched");

CompletableFuture<String> uploadFuture = logManager.uploadLogAsync();
uploadFuture.thenAccept(link -> System.out.println("Share: " + link));
```

### 6. **RepairManager**
Orchestrates validation and repair of all components.

**Features:**
- Registers and manages multiple validators
- Validates all components
- Repairs invalid components
- Asynchronous operations
- Comprehensive reporting
- User-friendly error messages

**Usage:**
```java
RepairManager repairManager = new RepairManager(logManager);

repairManager.registerValidator(libraryManager);
repairManager.registerValidator(nativesManager);

if (!repairManager.validateAll()) {
    repairManager.repairAll();
}

repairManager.printValidationSummary();
```

## Directory Structure

```
~/.integrityLauncher/
├── libraries/          # Game libraries and JARs
├── assets/            # Game assets
├── natives/           # Extracted native libraries
├── runtime/           # Java runtime installations
│   ├── java8/
│   ├── java17/
│   └── java21/
├── logs/              # Game logs
│   └── latest.log
├── temp/              # Temporary files
└── versions/          # Minecraft version JARs
```

## Configuration

Edit `SelfHealingConfig.java` to customize:

```java
// Directory paths
GAME_DIR, LIBRARIES_DIR, NATIVES_DIR, RUNTIME_DIR, LOGS_DIR

// Timeouts (milliseconds)
CONNECTION_TIMEOUT = 10_000
READ_TIMEOUT = 30_000

// Retry settings
MAX_RETRIES = 3

// Logging
MAX_LOG_SIZE_BYTES = 10 * 1024 * 1024
LOG_FILENAME = "latest.log"
```

## Launch Workflow

1. **Initialize:** Create directory structure and load config
2. **Validate:** Check all components (libraries, natives, Java)
3. **Repair:** Auto-repair any invalid components
4. **Select Java:** Detect or download required Java version
5. **Capture Logs:** Start capturing game process output
6. **Launch:** Start Minecraft with proper configuration
7. **Upload:** Optional log upload to mclo.gs for sharing

## Example: Complete Launch Sequence

```java
// Initialize config
SelfHealingConfig.initializeDirectories();
SelfHealingConfig.printConfiguration();

// Create components
LogManager logManager = new LogManager(logsDir, 10 * 1024 * 1024);
LibraryManager libraryManager = new LibraryManager(...);
NativesManager nativesManager = new NativesManager(...);
JavaManager javaManager = new JavaManager(...);

// Setup repair manager
RepairManager repairManager = new RepairManager(logManager);
repairManager.registerValidator(libraryManager);
repairManager.registerValidator(nativesManager);

// Validate and repair
if (!repairManager.validateAndRepairIfNeeded()) {
    System.out.println("Repair failed - cannot launch");
    System.exit(1);
}

// Select Java
String javaPath = javaManager.selectJavaForMinecraft("1.20.1");

// Launch game
Process game = launchMinecraft(javaPath);
logManager.captureProcessLogs(game);

// Optional: upload logs
logManager.uploadLogAsync().thenAccept(link -> {
    System.out.println("Share logs: " + link);
});
```

## Error Handling

All components include comprehensive error handling:

- **Validation errors:** Logged and trigger repair
- **Download failures:** Automatic retry (max 3 attempts by default)
- **Extraction failures:** Removed corrupted files and retry
- **Java issues:** Auto-download from Adoptium
- **Network errors:** Timeout handling with retry logic

## Performance Considerations

- **Asynchronous operations:** Prevent UI freezing
- **Caching:** Directory listings cached during validation
- **Streaming downloads:** Efficient memory usage for large files
- **Log rotation:** Prevents unlimited log file growth
- **Parallel repairs:** Multiple validators can repair concurrently

## API Reference

### LibraryManager
- `validate()` - Check all libraries
- `repair()` - Repair corrupted libraries
- `downloadLibrary(url, path, sha1)` - Download with verification
- `calculateSha1(file)` - Get file hash
- `getLibraryStatistics()` - Stats about libraries

### NativesManager
- `validate()` - Check native files
- `repair()` - Re-extract natives
- `cleanAndRepair()` - Full clean extraction
- `getOsIdentifier()` - Current OS type

### JavaManager
- `detectInstalledJava()` - Find installed Java versions
- `selectJavaForMinecraft(version)` - Get path for version
- `downloadAndInstallJava(version)` - Auto-download from Adoptium
- `validateJavaVersion(path, version)` - Verify Java version

### LogManager
- `captureProcessLogs(process)` - Start capturing output
- `logInfo(message)` - Log info message
- `logError(message)` - Log error message
- `uploadLogAsync()` - Upload to mclo.gs
- `rotateLog()` - Create new log file
- `close()` - Cleanup resources

### RepairManager
- `registerValidator(validator)` - Register component
- `validateAll()` - Check all components
- `repairAll()` - Repair all invalid components
- `validateAndRepairIfNeeded()` - Full workflow
- `printValidationSummary()` - Print status report

## mclo.gs Integration

The LogManager automatically uploads logs to mclo.gs:

```
POST https://mclo.gs/api/logs
Content-Type: multipart/form-data

Response: {"success":true,"id":"abc123"}
Returns: https://mclo.gs/abc123
```

## Troubleshooting

### "Native files not found"
→ Check `NativesManager.repair()` or run clean extraction

### "Java version mismatch"
→ `JavaManager` will auto-download correct version

### "Library download failed"
→ Check network connection; retry logic runs automatically

### "Log upload failed"
→ Check internet connection; logs still saved locally

## Performance Metrics

- Library validation: ~100ms for typical installation
- Native extraction: ~500ms-2s depending on size
- Java download: ~1-3 minutes (first time only)
- Log upload: ~1-5 seconds depending on size

## Future Enhancements

- [ ] Parallel library downloads
- [ ] Incremental asset validation
- [ ] Java 23+ support
- [ ] Custom log parsers
- [ ] Statistics dashboard
- [ ] Automatic cleanup of old logs

---

**Author:** Integrity Launcher Development Team  
**Version:** 1.0.0  
**Last Updated:** 2026-04-18
