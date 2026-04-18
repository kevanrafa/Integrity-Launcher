package com.integritylauncher.selfhealing;

import java.io.*;
import java.nio.charset.StandardCharsets;

/**
 * Integration utilities for connecting the Self-Healing System with the Rust launcher backend.
 * Provides methods for serializing component states and communicating with the launcher.
 */
public class LauncherIntegration {
    
    /**
     * Represents the state of the self-healing system for serialization.
     */
    public static class SystemState {
        public boolean librariesValid;
        public boolean nativesValid;
        public boolean javaAvailable;
        public String selectedJavaPath;
        public String minecraftVersion;
        public String logFilePath;
        public long timestamp;
        
        public SystemState() {
            this.timestamp = System.currentTimeMillis();
        }
        
        @Override
        public String toString() {
            return "SystemState{" +
                    "librariesValid=" + librariesValid +
                    ", nativesValid=" + nativesValid +
                    ", javaAvailable=" + javaAvailable +
                    ", selectedJavaPath='" + selectedJavaPath + '\'' +
                    ", minecraftVersion='" + minecraftVersion + '\'' +
                    ", logFilePath='" + logFilePath + '\'' +
                    ", timestamp=" + timestamp +
                    '}';
        }
    }
    
    /**
     * Captures the current state of the self-healing system.
     */
    public static SystemState captureSystemState(SelfHealingSystem system) throws Exception {
        SystemState state = new SystemState();
        
        // Note: Would need to add getters to SelfHealingSystem for actual use
        // state.librariesValid = system.libraryManager.validate();
        // state.nativesValid = system.nativesManager.validate();
        state.logFilePath = system.getLogFilePath().toString();
        
        return state;
    }
    
    /**
     * Writes system state to a file for inter-process communication.
     */
    public static void writeStateFile(String stateFilePath, SystemState state) throws IOException {
        try (PrintWriter writer = new PrintWriter(
                new OutputStreamWriter(new FileOutputStream(stateFilePath), StandardCharsets.UTF_8))) {
            writer.println("# System State - " + state.timestamp);
            writer.println("libraries_valid=" + state.librariesValid);
            writer.println("natives_valid=" + state.nativesValid);
            writer.println("java_available=" + state.javaAvailable);
            writer.println("selected_java=" + (state.selectedJavaPath != null ? state.selectedJavaPath : ""));
            writer.println("minecraft_version=" + (state.minecraftVersion != null ? state.minecraftVersion : ""));
            writer.println("log_file=" + state.logFilePath);
        }
    }
    
    /**
     * Reads system state from a file.
     */
    public static SystemState readStateFile(String stateFilePath) throws IOException {
        SystemState state = new SystemState();
        
        try (BufferedReader reader = new BufferedReader(
                new InputStreamReader(new FileInputStream(stateFilePath), StandardCharsets.UTF_8))) {
            String line;
            while ((line = reader.readLine()) != null) {
                if (line.startsWith("#") || line.isEmpty()) continue;
                
                String[] parts = line.split("=", 2);
                if (parts.length != 2) continue;
                
                String key = parts[0].trim();
                String value = parts[1].trim();
                
                switch (key) {
                    case "libraries_valid":
                        state.librariesValid = Boolean.parseBoolean(value);
                        break;
                    case "natives_valid":
                        state.nativesValid = Boolean.parseBoolean(value);
                        break;
                    case "java_available":
                        state.javaAvailable = Boolean.parseBoolean(value);
                        break;
                    case "selected_java":
                        state.selectedJavaPath = value.isEmpty() ? null : value;
                        break;
                    case "minecraft_version":
                        state.minecraftVersion = value.isEmpty() ? null : value;
                        break;
                    case "log_file":
                        state.logFilePath = value;
                        break;
                }
            }
        }
        
        return state;
    }
    
    /**
     * Validation result for communication with the launcher.
     */
    public static class ValidationResult {
        public boolean success;
        public String message;
        public RepairManager.RepairReport report;
        public long elapsedMs;
        
        public ValidationResult(boolean success, String message, RepairManager.RepairReport report, long elapsedMs) {
            this.success = success;
            this.message = message;
            this.report = report;
            this.elapsedMs = elapsedMs;
        }
        
        public String toJson() {
            StringBuilder sb = new StringBuilder();
            sb.append("{");
            sb.append("\"success\":").append(success).append(",");
            sb.append("\"message\":\"").append(escapeJson(message)).append("\",");
            sb.append("\"elapsed_ms\":").append(elapsedMs).append(",");
            sb.append("\"report\":{");
            sb.append("\"total\":").append(report.getTotalValidators()).append(",");
            sb.append("\"valid\":").append(report.getValidCount()).append(",");
            sb.append("\"invalid\":").append(report.getInvalidCount());
            sb.append("}");
            sb.append("}");
            return sb.toString();
        }
        
        private String escapeJson(String str) {
            return str.replace("\"", "\\\"").replace("\n", "\\n").replace("\r", "\\r");
        }
    }
    
    /**
     * Runs the complete validation workflow and returns results.
     */
    public static ValidationResult validateSystemForLaunch(SelfHealingSystem system) {
        long startTime = System.currentTimeMillis();
        
        try {
            boolean valid = system.validateAndRepairIfNeeded();
            RepairManager.RepairReport report = system.repairManager.getValidationReport();
            
            long elapsedMs = System.currentTimeMillis() - startTime;
            String message = valid ? "System ready for launch" : "System validation/repair failed";
            
            return new ValidationResult(valid, message, report, elapsedMs);
        } catch (Exception e) {
            long elapsedMs = System.currentTimeMillis() - startTime;
            return new ValidationResult(false, "Exception: " + e.getMessage(), null, elapsedMs);
        }
    }
    
    /**
     * Prepares the launcher for game launch.
     */
    public static class LaunchPreperation {
        public boolean success;
        public String javaPath;
        public String message;
        public java.util.Map<String, String> environment;
        
        public LaunchPreperation() {
            this.environment = new java.util.HashMap<>();
        }
    }
    
    /**
     * Prepares everything for launching Minecraft.
     */
    public static LaunchPreperation prepareLaunch(SelfHealingSystem system, String minecraftVersion) {
        LaunchPreperation prep = new LaunchPreperation();
        
        try {
            // Validate all systems
            if (!system.validateAndRepairIfNeeded()) {
                prep.success = false;
                prep.message = "System validation failed";
                return prep;
            }
            
            // Select Java
            prep.javaPath = system.selectJavaRuntime(minecraftVersion);
            if (prep.javaPath == null || prep.javaPath.isEmpty()) {
                prep.success = false;
                prep.message = "Failed to select Java runtime";
                return prep;
            }
            
            // Setup environment
            prep.environment.put("JAVA_EXECUTABLE", prep.javaPath);
            prep.environment.put("NATIVES_DIR", SelfHealingConfig.NATIVES_DIR);
            prep.environment.put("LIBRARIES_DIR", SelfHealingConfig.LIBRARIES_DIR);
            prep.environment.put("ASSETS_DIR", SelfHealingConfig.ASSETS_DIR);
            prep.environment.put("LOG_FILE", system.getLogFilePath().toString());
            
            prep.success = true;
            prep.message = "Launch preparation complete";
            
        } catch (Exception e) {
            prep.success = false;
            prep.message = "Error: " + e.getMessage();
        }
        
        return prep;
    }
    
    /**
     * Example integration with the Rust launcher.
     */
    public static void main(String[] args) {
        try {
            System.out.println("[Integration] Starting Integrity Launcher with Self-Healing System");
            
            // Parse arguments
            String command = args.length > 0 ? args[0] : "launch";
            String minecraftVersion = args.length > 1 ? args[1] : "1.20.1";
            
            // Initialize system
            SelfHealingSystem system = new SelfHealingSystem();
            
            switch (command) {
                case "validate":
                    // Validate only
                    ValidationResult result = validateSystemForLaunch(system);
                    System.out.println(result.toJson());
                    System.exit(result.success ? 0 : 1);
                    break;
                    
                case "prepare":
                    // Prepare for launch
                    LaunchPreperation prep = prepareLaunch(system, minecraftVersion);
                    System.out.println("JAVA_PATH=" + prep.javaPath);
                    System.out.println("SUCCESS=" + prep.success);
                    System.out.println("MESSAGE=" + prep.message);
                    System.exit(prep.success ? 0 : 1);
                    break;
                    
                case "launch":
                default:
                    // Full launch workflow
                    LaunchPreperation launchPrep = prepareLaunch(system, minecraftVersion);
                    if (!launchPrep.success) {
                        System.err.println("Launch preparation failed: " + launchPrep.message);
                        System.exit(1);
                    }
                    
                    System.out.println("System ready to launch Minecraft " + minecraftVersion);
                    System.out.println("Java: " + launchPrep.javaPath);
                    
                    // The launcher would now fork a process here
                    // Process game = Runtime.getRuntime().exec(...);
                    // system.captureGameLogs(game);
                    
                    System.exit(0);
            }
            
        } catch (Exception e) {
            System.err.println("[Integration] Error: " + e.getMessage());
            e.printStackTrace();
            System.exit(1);
        }
    }
}
