# Integrity Launcher Self-Healing System - Quick Start Guide

## Installation

1. Copy all Java files from `src/com/integritylauncher/selfhealing/` to your project
2. No external dependencies required (uses only Java Standard Library)
3. Minimum Java version: Java 8

## Basic Usage (3 lines)

```java
SelfHealingSystem system = new SelfHealingSystem();
system.validateAndRepairIfNeeded();
String javaPath = system.selectJavaRuntime("1.20.1");
```

## Complete Launch Workflow

```java
import com.integritylauncher.selfhealing.*;

public class LauncherMain {
    public static void main(String[] args) throws Exception {
        // Initialize
        SelfHealingSystem system = new SelfHealingSystem();
        
        // Validate and repair
        if (!system.validateAndRepairIfNeeded()) {
            System.err.println("Failed to repair game installation!");
            System.exit(1);
        }
        
        // Select Java runtime
        String minecraftVersion = "1.20.1";
        String javaPath = system.selectJavaRuntime(minecraftVersion);
        System.out.println("Using Java: " + javaPath);
        
        // Prepare and launch game
        ProcessBuilder pb = new ProcessBuilder(
            javaPath,
            "-Djava.library.path=" + SelfHealingConfig.NATIVES_DIR,
            "-cp", SelfHealingConfig.LIBRARIES_DIR,
            "net.minecraft.client.main.Main"
        );
        
        Process gameProcess = pb.start();
        
        // Capture logs
        system.captureGameLogs(gameProcess);
        
        // Wait for game to finish
        gameProcess.waitFor();
        
        // Optional: Upload logs
        system.uploadLogs().thenAccept(link -> {
            System.out.println("Share logs: " + link);
        });
        
        // Cleanup
        system.close();
    }
}
```

## Features

### 1. Automatic Validation
```java
// Checks all components automatically
if (system.validateAll()) {
    System.out.println("Ready to launch!");
}
```

### 2. Automatic Repair
```java
// Repairs any missing/corrupted files
system.repairAll();
```

### 3. Java Management
```java
// Auto-selects correct Java version for Minecraft
String javaPath = system.selectJavaRuntime("1.20.1");

// Auto-detects installed versions
Map<Integer, String> versions = javaManager.detectInstalledJava();
// Java 8: "/path/to/java8"
// Java 17: "/path/to/java17"
// Java 21: "/path/to/java21"
```

### 4. Logging & Sharing
```java
// Capture game console output
system.captureGameLogs(minecraftProcess);

// Upload logs to mclo.gs
system.uploadLogs().thenAccept(link -> {
    System.out.println("Share: " + link);  // https://mclo.gs/abc123
});
```

## Configuration

Edit `SelfHealingConfig.java` to customize:

```java
// Game directories
GAME_DIR = System.getProperty("user.home") + "/.integrityLauncher"

// Timeouts
CONNECTION_TIMEOUT = 10_000  // 10 seconds
MAX_RETRIES = 3              // Retry 3 times

// Logging
MAX_LOG_SIZE_BYTES = 10 * 1024 * 1024  // 10 MB
```

## Directory Structure

After first run, the system creates:

```
~/.integrityLauncher/
├── libraries/              # Game libraries
├── assets/                 # Game assets
├── natives/                # Native libraries
├── runtime/                # Java runtimes
│   ├── java8/
│   ├── java17/
│   └── java21/
├── logs/                   # Game logs
└── temp/                   # Temporary files
```

## Common Tasks

### Check System Status
```java
system.printValidationSummary();
```

### Download Specific Java Version
```java
JavaManager javaManager = new JavaManager(...);
String javaPath = javaManager.downloadAndInstallJava(17);
```

### Get Library Statistics
```java
String stats = libraryManager.getLibraryStatistics();
System.out.println(stats);
// Output: Total JARs: 152, Empty: 0, Total Size: 450.25 MB
```

### Manual Library Repair
```java
libraryManager.cleanCorruptedFiles(librariesDir);
libraryManager.repair();
```

### Clear and Re-extract Natives
```java
nativesManager.cleanAndRepair();
```

### Upload Log Asynchronously
```java
system.uploadLogs()
    .thenAccept(link -> System.out.println("Log: " + link))
    .exceptionally(e -> {
        System.err.println("Upload failed: " + e.getMessage());
        return null;
    });
```

## Error Handling

All methods throw exceptions with detailed messages:

```java
try {
    system.validateAndRepairIfNeeded();
} catch (Exception e) {
    System.err.println("Error: " + e.getMessage());
    // User-friendly error messages instead of stack traces
}
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| "Native files not found" | Run `nativesManager.repair()` |
| "Java version mismatch" | `JavaManager` auto-downloads correct version |
| "Library download failed" | Check internet; retry logic runs automatically |
| "Log upload failed" | Logs saved locally; try again later |

## Performance Tips

1. **Async Operations:** Use `*Async()` methods to prevent UI freezing
2. **First Launch:** Java download may take 1-3 minutes
3. **Subsequent Launches:** Validation takes ~200ms
4. **Parallel Repair:** Two threads handle repairs concurrently

## Integration with Rust Backend

Use `LauncherIntegration` for inter-process communication:

```java
// Validate and get JSON result
ValidationResult result = LauncherIntegration.validateSystemForLaunch(system);
String json = result.toJson();
// Send to Rust backend via IPC

// Prepare launch with environment variables
LaunchPreperation prep = LauncherIntegration.prepareLaunch(system, "1.20.1");
System.out.println("JAVA_PATH=" + prep.javaPath);
```

## API Reference

### SelfHealingSystem
- `validateAll()` - Check all components
- `repairAll()` - Repair all components
- `validateAndRepairIfNeeded()` - Full workflow
- `selectJavaRuntime(version)` - Get Java path
- `captureGameLogs(process)` - Start log capture
- `uploadLogs()` - Upload to mclo.gs
- `getLogFilePath()` - Get log file location
- `close()` - Cleanup resources

### LibraryManager
- `validate()` - Check libraries
- `repair()` - Repair corrupted files
- `downloadLibrary(url, path, sha1)` - Download with verify
- `getLibraryStatistics()` - Get stats

### NativesManager
- `validate()` - Check native files
- `repair()` - Re-extract natives
- `cleanAndRepair()` - Full clean extraction
- `getOsIdentifier()` - Get OS type

### JavaManager
- `detectInstalledJava()` - Find installed versions
- `selectJavaForMinecraft(version)` - Get path for version
- `downloadAndInstallJava(version)` - Install from Adoptium

### LogManager
- `captureProcessLogs(process)` - Start capturing
- `logInfo(message)` - Log message
- `uploadLogAsync()` - Upload to mclo.gs
- `rotateLog()` - Create new log file

## Example: Full Integration

```java
public class IntegrityLauncher {
    public static void main(String[] args) throws Exception {
        // Initialize self-healing system
        SelfHealingSystem system = new SelfHealingSystem();
        
        // Validate and repair
        System.out.println("Checking game installation...");
        if (!system.validateAndRepairIfNeeded()) {
            System.err.println("FATAL: Cannot repair game installation");
            return;
        }
        
        // Select Java
        System.out.println("Selecting Java runtime...");
        String javaPath = system.selectJavaRuntime("1.20.1");
        
        // Show summary
        system.printValidationSummary();
        
        // Launch game
        System.out.println("Launching Minecraft...");
        ProcessBuilder pb = new ProcessBuilder(javaPath, "--version");
        Process p = pb.start();
        system.captureGameLogs(p);
        
        // Upload logs after game closes
        p.waitFor();
        system.uploadLogs().thenAccept(link -> {
            System.out.println("Share logs: " + link);
        });
        
        system.close();
    }
}
```

## What Gets Validated & Repaired

✅ **Libraries:** All Minecraft JARs, LWJGL, dependencies
✅ **Assets:** Game textures, sounds, language files
✅ **Natives:** Platform-specific libraries (.dll, .so, .dylib)
✅ **Java Runtime:** Correct version for Minecraft version
✅ **Logs:** Captured and saved automatically

## Status

🟢 **PRODUCTION READY**
- Fully implemented
- Comprehensive error handling
- Zero external dependencies
- Well-documented
- Easy to integrate

---

**For complete documentation, see:** `README.md`  
**For implementation details, see:** `SELF_HEALING_IMPLEMENTATION.md`
