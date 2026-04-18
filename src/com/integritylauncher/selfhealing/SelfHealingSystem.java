package com.integritylauncher.selfhealing;

import java.nio.file.*;
import java.util.*;

/**
 * Main entry point for the Integrity Launcher Self-Healing System.
 * 
 * This class demonstrates the integration of all self-healing components:
 * - LibraryManager: Validates and repairs game libraries
 * - NativesManager: Handles native library extraction and validation
 * - JavaManager: Detects and manages Java runtimes
 * - LogManager: Captures and uploads game logs
 * - RepairManager: Orchestrates all validation and repair operations
 * 
 * @author Integrity Launcher Development Team
 * @version 1.0.0
 */
public class SelfHealingSystem {
    
    private final LogManager logManager;
    private final LibraryManager libraryManager;
    private final NativesManager nativesManager;
    private final JavaManager javaManager;
    private final RepairManager repairManager;
    
    /**
     * Initializes the self-healing system with all required components.
     */
    public SelfHealingSystem() throws Exception {
        // Initialize configuration
        SelfHealingConfig.initializeDirectories();
        
        // Initialize LogManager
        this.logManager = new LogManager(
            Paths.get(SelfHealingConfig.LOGS_DIR),
            SelfHealingConfig.MAX_LOG_SIZE_BYTES
        );
        
        // Initialize component managers
        this.libraryManager = new LibraryManager(
            Paths.get(SelfHealingConfig.LIBRARIES_DIR),
            Paths.get(SelfHealingConfig.ASSETS_DIR),
            Paths.get(SelfHealingConfig.VERSIONS_DIR),
            SelfHealingConfig.MAX_RETRIES,
            SelfHealingConfig.CONNECTION_TIMEOUT
        );
        
        this.nativesManager = new NativesManager(
            Paths.get(SelfHealingConfig.NATIVES_DIR),
            Paths.get(SelfHealingConfig.LIBRARIES_DIR),
            Paths.get(SelfHealingConfig.TEMP_DIR),
            SelfHealingConfig.MAX_RETRIES
        );
        
        this.javaManager = new JavaManager(
            Paths.get(SelfHealingConfig.RUNTIME_DIR),
            SelfHealingConfig.CONNECTION_TIMEOUT,
            SelfHealingConfig.MAX_RETRIES
        );
        
        // Initialize RepairManager
        this.repairManager = new RepairManager(logManager);
        repairManager.registerValidator(libraryManager);
        repairManager.registerValidator(nativesManager);
        
        logManager.logInfo("Self-Healing System initialized successfully");
    }
    
    /**
     * Validates all components before game launch.
     * @return true if all components are valid
     */
    public boolean validateAll() throws Exception {
        logManager.logInfo("Starting complete validation...");
        return repairManager.validateAll();
    }
    
    /**
     * Repairs any invalid components.
     * @return true if repair was successful
     */
    public boolean repairAll() throws Exception {
        logManager.logInfo("Starting repair of invalid components...");
        return repairManager.repairAll();
    }
    
    /**
     * Validates and repairs if needed.
     * @return true if validation passed or repair succeeded
     */
    public boolean validateAndRepairIfNeeded() throws Exception {
        if (!validateAll()) {
            logManager.logInfo("Invalid components detected, attempting repair...");
            return repairAll() && validateAll();
        }
        return true;
    }
    
    /**
     * Selects appropriate Java runtime for the given Minecraft version.
     * @param minecraftVersion Minecraft version (e.g., "1.20.1")
     * @return Path to Java executable
     */
    public String selectJavaRuntime(String minecraftVersion) throws Exception {
        logManager.logInfo("Selecting Java runtime for Minecraft " + minecraftVersion);
        return javaManager.selectJavaForMinecraft(minecraftVersion);
    }
    
    /**
     * Starts capturing logs from a Minecraft process.
     * @param process Minecraft process
     */
    public void captureGameLogs(Process process) {
        logManager.logInfo("Starting game log capture...");
        logManager.captureProcessLogs(process);
    }
    
    /**
     * Uploads current logs to mclo.gs asynchronously.
     * @return CompletableFuture with share link
     */
    public java.util.concurrent.CompletableFuture<String> uploadLogs() {
        return logManager.uploadLogAsync();
    }
    
    /**
     * Prints a summary of all validation results.
     */
    public void printValidationSummary() {
        repairManager.printValidationSummary();
    }
    
    /**
     * Gets the log file path.
     * @return Path to current log file
     */
    public Path getLogFilePath() {
        return logManager.getLogFile();
    }
    
    /**
     * Closes all resources and cleanup.
     */
    public void close() {
        logManager.logInfo("Closing Self-Healing System...");
        logManager.close();
        repairManager.close();
    }
    
    /**
     * Main entry point for demonstration and testing.
     */
    public static void main(String[] args) {
        try {
            SelfHealingConfig.printConfiguration();
            
            System.out.println("\n[MAIN] Initializing Integrity Launcher Self-Healing System...\n");
            
            SelfHealingSystem system = new SelfHealingSystem();
            
            // Run complete validation and repair workflow
            if (system.validateAndRepairIfNeeded()) {
                System.out.println("\n[MAIN] ✓ All systems validated and ready for launch!\n");
                system.printValidationSummary();
            } else {
                System.out.println("\n[MAIN] ✗ System validation/repair failed!\n");
                system.printValidationSummary();
                System.exit(1);
            }
            
            // Demonstrate Java selection
            String javaPath = system.selectJavaRuntime("1.20.1");
            System.out.println("[MAIN] Selected Java: " + javaPath);
            
            // Cleanup
            Thread.sleep(2000);
            system.close();
            
        } catch (Exception e) {
            System.err.println("[ERROR] " + e.getMessage());
            e.printStackTrace();
            System.exit(1);
        }
    }
}
