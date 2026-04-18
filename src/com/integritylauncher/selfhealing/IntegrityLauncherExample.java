package com.integritylauncher.selfhealing;

import java.nio.file.*;
import java.util.*;

/**
 * Example implementation and usage guide for the Integrity Launcher Self-Healing System.
 */
public class IntegrityLauncherExample {
    public static void main(String[] args) {
        try {
            // Initialize directory structure
            Path gameDir = Paths.get(System.getProperty("user.home"), ".integrityLauncher");
            Path librariesDir = gameDir.resolve("libraries");
            Path assetsDir = gameDir.resolve("assets");
            Path nativesDir = gameDir.resolve("natives");
            Path runtimeDir = gameDir.resolve("runtime");
            Path logsDir = gameDir.resolve("logs");
            Path tempDir = gameDir.resolve("temp");

            System.out.println("=".repeat(80));
            System.out.println("Integrity Launcher - Self-Healing System Example");
            System.out.println("=".repeat(80));

            // Initialize LogManager
            System.out.println("\n[INIT] Initializing LogManager...");
            LogManager logManager = new LogManager(logsDir, 10 * 1024 * 1024); // 10 MB max
            logManager.logInfo("=== Integrity Launcher Session Started ===");

            // Initialize components
            System.out.println("[INIT] Initializing LibraryManager...");
            LibraryManager libraryManager = new LibraryManager(
                    librariesDir, assetsDir, gameDir.resolve("versions"), 3, 10000);

            System.out.println("[INIT] Initializing NativesManager...");
            NativesManager nativesManager = new NativesManager(
                    nativesDir, librariesDir, tempDir, 3);

            System.out.println("[INIT] Initializing JavaManager...");
            JavaManager javaManager = new JavaManager(runtimeDir, 10000, 3);

            // Create RepairManager and register validators
            System.out.println("[INIT] Initializing RepairManager...");
            RepairManager repairManager = new RepairManager(logManager);
            repairManager.registerValidator(libraryManager);
            repairManager.registerValidator(nativesManager);

            // Validate all components before launch
            System.out.println("\n[VALIDATION] Validating all components...");
            repairManager.printValidationSummary();

            // Attempt repair if validation fails
            if (!repairManager.validateAll()) {
                System.out.println("\n[REPAIR] Invalid components detected, attempting repair...");
                logManager.logInfo("Starting automatic repair...");
                
                if (repairManager.repairAll()) {
                    System.out.println("[REPAIR] Repair completed!");
                    logManager.logInfo("Repair completed successfully");
                    
                    // Validate again after repair
                    if (repairManager.validateAll()) {
                        System.out.println("[VALIDATION] All components now valid!");
                        logManager.logInfo("Post-repair validation passed");
                    } else {
                        System.out.println("[ERROR] Components still invalid after repair");
                        logManager.logError("Post-repair validation failed");
                    }
                } else {
                    System.out.println("[ERROR] Repair failed!");
                    logManager.logError("Repair failed - cannot launch");
                    System.exit(1);
                }
            }

            // Select and validate Java runtime
            System.out.println("\n[JAVA] Detecting and selecting Java runtime...");
            String minecraftVersion = "1.20.1";
            javaManager.validateAllJavaInstallations();
            
            try {
                String javaPath = javaManager.selectJavaForMinecraft(minecraftVersion);
                System.out.println("[JAVA] Selected Java: " + javaPath);
                logManager.logInfo("Java selected: " + javaPath);
            } catch (Exception e) {
                System.out.println("[ERROR] Failed to select Java: " + e.getMessage());
                logManager.logError("Java selection failed: " + e.getMessage());
            }

            // Get library statistics
            System.out.println("\n[LIBRARIES] " + libraryManager.getLibraryStatistics());

            // Example: Upload log asynchronously
            System.out.println("\n[LOGGING] Log file location: " + logManager.getLogFile());
            System.out.println("[LOGGING] Starting asynchronous log upload...");
            
            logManager.uploadLogAsync()
                    .thenAccept(shareLink -> {
                        if (shareLink != null) {
                            System.out.println("[LOGGING] Log uploaded! Share link: " + shareLink);
                            logManager.logInfo("Log uploaded: " + shareLink);
                        } else {
                            System.out.println("[LOGGING] Log upload failed");
                        }
                    })
                    .exceptionally(e -> {
                        System.out.println("[LOGGING] Log upload error: " + e.getMessage());
                        return null;
                    });

            // Simulate game launch (this would normally happen here)
            System.out.println("\n[LAUNCH] All systems ready! Game would launch here.");
            logManager.logInfo("System ready for launch");

            // Example of game process logging
            System.out.println("[LAUNCH] Simulating game process output capture...");
            // In real launcher: logManager.captureProcessLogs(minecraftProcess);
            
            System.out.println("\n=".repeat(80));
            System.out.println("Self-Healing System Ready!");
            System.out.println("=".repeat(80));

            // Clean up
            Thread.sleep(2000); // Allow async operations to complete
            logManager.close();
            repairManager.close();

        } catch (Exception e) {
            System.out.println("[ERROR] Fatal error: " + e.getMessage());
            e.printStackTrace();
            System.exit(1);
        }
    }

    /**
     * Example of how to use individual components.
     */
    public static void exampleComponentUsage() throws Exception {
        Path gameDir = Paths.get(System.getProperty("user.home"), ".integrityLauncher");

        // Example 1: Using LogManager
        LogManager logManager = new LogManager(gameDir.resolve("logs"), 10 * 1024 * 1024);
        logManager.logInfo("Application started");
        logManager.logError("Example error message");

        // Example 2: Using LibraryManager
        LibraryManager libraryManager = new LibraryManager(
                gameDir.resolve("libraries"),
                gameDir.resolve("assets"),
                gameDir.resolve("versions"),
                3, 10000);
        
        if (libraryManager.validate()) {
            System.out.println("Libraries are valid");
        } else {
            System.out.println("Libraries need repair");
            libraryManager.repair();
        }

        // Example 3: Using NativesManager
        NativesManager nativesManager = new NativesManager(
                gameDir.resolve("natives"),
                gameDir.resolve("libraries"),
                gameDir.resolve("temp"),
                3);
        
        String osType = nativesManager.getOsIdentifier();
        System.out.println("Detected OS: " + osType);

        // Example 4: Using JavaManager
        JavaManager javaManager = new JavaManager(gameDir.resolve("runtime"), 10000, 3);
        Map<Integer, String> javaVersions = javaManager.detectInstalledJava();
        System.out.println("Found Java versions: " + javaVersions.keySet());

        logManager.close();
    }

    /**
     * Example of async repair workflow.
     */
    public static void exampleAsyncRepair() throws Exception {
        Path gameDir = Paths.get(System.getProperty("user.home"), ".integrityLauncher");
        
        LogManager logManager = new LogManager(gameDir.resolve("logs"), 10 * 1024 * 1024);
        RepairManager repairManager = new RepairManager(logManager);
        
        // Register validators
        repairManager.registerValidator(new LibraryManager(
                gameDir.resolve("libraries"),
                gameDir.resolve("assets"),
                gameDir.resolve("versions"),
                3, 10000));

        // Validate and repair asynchronously
        repairManager.validateAndRepairIfNeededAsync()
                .thenAccept(success -> {
                    if (success) {
                        System.out.println("All systems ready!");
                    } else {
                        System.out.println("Repair failed!");
                    }
                })
                .exceptionally(e -> {
                    System.out.println("Error: " + e.getMessage());
                    return null;
                });

        // Wait for async operations to complete
        Thread.sleep(5000);
        
        logManager.close();
        repairManager.close();
    }
}
