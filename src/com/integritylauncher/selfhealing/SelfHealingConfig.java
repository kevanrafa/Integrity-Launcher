package com.integritylauncher.selfhealing;

import java.nio.file.*;

/**
 * Configuration and constants for the self-healing system.
 */
public class SelfHealingConfig {
    // Directory Configuration
    public static final String GAME_DIR = System.getProperty("user.home") + "/.integrityLauncher";
    public static final String LIBRARIES_DIR = GAME_DIR + "/libraries";
    public static final String ASSETS_DIR = GAME_DIR + "/assets";
    public static final String NATIVES_DIR = GAME_DIR + "/natives";
    public static final String RUNTIME_DIR = GAME_DIR + "/runtime";
    public static final String LOGS_DIR = GAME_DIR + "/logs";
    public static final String TEMP_DIR = GAME_DIR + "/temp";
    public static final String VERSIONS_DIR = GAME_DIR + "/versions";

    // Timeout Configuration (in milliseconds)
    public static final int CONNECTION_TIMEOUT = 10_000;
    public static final int READ_TIMEOUT = 30_000;
    public static final int MAX_RETRIES = 3;

    // Logging Configuration
    public static final int MAX_LOG_SIZE_BYTES = 10 * 1024 * 1024; // 10 MB
    public static final String LOG_FILENAME = "latest.log";

    // API Endpoints
    public static final String MCLO_GS_API = "https://mclo.gs/api/";
    public static final String ADOPTIUM_API = "https://api.adoptium.net/v3/";

    // Java Version Configuration
    public static final int LEGACY_JAVA_VERSION = 8;      // Pre-1.17
    public static final int MID_JAVA_VERSION = 17;        // 1.17-1.19
    public static final int LATEST_JAVA_VERSION = 21;     // 1.20+

    // Minecraft Version Ranges
    public static final String MC_LEGACY_CUTOFF = "1.17";  // Java 8 for versions before this
    public static final String MC_MID_CUTOFF = "1.20";     // Java 17 for versions before this
    public static final String MC_LATEST_CUTOFF = "2.0";   // Java 21+ for versions after/at this

    // Native File Extensions per Platform
    public static final String[] WINDOWS_NATIVE_EXTS = {".dll"};
    public static final String[] LINUX_NATIVE_EXTS = {".so"};
    public static final String[] MACOS_NATIVE_EXTS = {".dylib", ".jnilib"};

    /**
     * Initializes all required directories.
     */
    public static void initializeDirectories() throws Exception {
        String[] dirs = {
            GAME_DIR, LIBRARIES_DIR, ASSETS_DIR, NATIVES_DIR,
            RUNTIME_DIR, LOGS_DIR, TEMP_DIR, VERSIONS_DIR
        };

        for (String dir : dirs) {
            Path path = Paths.get(dir);
            Files.createDirectories(path);
            System.out.println("[Config] Created/verified directory: " + path);
        }
    }

    /**
     * Gets directory for a specific Java version.
     */
    public static Path getJavaRuntimeDir(int majorVersion) {
        return Paths.get(RUNTIME_DIR, "java" + majorVersion);
    }

    /**
     * Prints configuration summary.
     */
    public static void printConfiguration() {
        System.out.println("\n" + "=".repeat(70));
        System.out.println("Self-Healing System Configuration");
        System.out.println("=".repeat(70));
        System.out.println("Game Directory: " + GAME_DIR);
        System.out.println("Libraries: " + LIBRARIES_DIR);
        System.out.println("Assets: " + ASSETS_DIR);
        System.out.println("Natives: " + NATIVES_DIR);
        System.out.println("Runtime: " + RUNTIME_DIR);
        System.out.println("Logs: " + LOGS_DIR);
        System.out.println("Connection Timeout: " + CONNECTION_TIMEOUT + "ms");
        System.out.println("Max Retries: " + MAX_RETRIES);
        System.out.println("Max Log Size: " + (MAX_LOG_SIZE_BYTES / (1024 * 1024)) + " MB");
        System.out.println("=".repeat(70) + "\n");
    }
}
