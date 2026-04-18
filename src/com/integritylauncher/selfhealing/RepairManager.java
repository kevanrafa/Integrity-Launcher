package com.integritylauncher.selfhealing;

import java.util.*;
import java.util.concurrent.*;

/**
 * Orchestrates validation and repair of all game components.
 * Manages multiple validators and coordinates repair operations.
 */
public class RepairManager {
    private final List<Validator> validators;
    private final ExecutorService executorService;
    private final LogManager logManager;

    public RepairManager(LogManager logManager) {
        this.validators = new ArrayList<>();
        this.logManager = logManager;
        this.executorService = Executors.newFixedThreadPool(2, r -> {
            Thread t = new Thread(r, "RepairManager-Worker");
            t.setDaemon(true);
            return t;
        });
    }

    /**
     * Registers a validator to be managed by this repair manager.
     * @param validator Validator to register
     */
    public void registerValidator(Validator validator) {
        validators.add(validator);
        logManager.logInfo("Registered validator: " + validator.getName());
    }

    /**
     * Registers multiple validators at once.
     * @param validatorList List of validators to register
     */
    public void registerValidators(List<Validator> validatorList) {
        for (Validator v : validatorList) {
            registerValidator(v);
        }
    }

    /**
     * Validates all registered components synchronously.
     * @return true if all components are valid
     */
    public boolean validateAll() {
        System.out.println("[RepairManager] Validating all components...");
        logManager.logInfo("Starting validation of all components");

        boolean allValid = true;

        for (Validator validator : validators) {
            try {
                String name = validator.getName();
                System.out.println("[RepairManager] Validating " + name + "...");
                logManager.logInfo("Validating: " + name);

                if (validator.validate()) {
                    System.out.println("[RepairManager] ✓ " + name + " is valid");
                    logManager.logInfo(name + " is VALID");
                } else {
                    System.out.println("[RepairManager] ✗ " + name + " is invalid");
                    logManager.logInfo(name + " is INVALID");
                    allValid = false;
                }
            } catch (Exception e) {
                System.out.println("[RepairManager] ✗ " + validator.getName() + " validation failed: " + e.getMessage());
                logManager.logError(validator.getName() + " validation failed: " + e.getMessage());
                allValid = false;
            }
        }

        return allValid;
    }

    /**
     * Repairs all invalid components synchronously.
     * @return true if repair was successful
     */
    public boolean repairAll() {
        System.out.println("[RepairManager] Starting repair of all components...");
        logManager.logInfo("Starting repair of all components");

        boolean repairSuccessful = true;

        for (Validator validator : validators) {
            try {
                String name = validator.getName();
                System.out.println("[RepairManager] Repairing " + name + "...");
                logManager.logInfo("Repairing: " + name);

                validator.repair();

                System.out.println("[RepairManager] ✓ " + name + " repaired");
                logManager.logInfo(name + " repair COMPLETED");
            } catch (Exception e) {
                System.out.println("[RepairManager] ✗ " + validator.getName() + " repair failed: " + e.getMessage());
                logManager.logError(validator.getName() + " repair failed: " + e.getMessage());
                repairSuccessful = false;
            }
        }

        return repairSuccessful;
    }

    /**
     * Validates all components asynchronously.
     * @return CompletableFuture with validation result
     */
    public CompletableFuture<Boolean> validateAllAsync() {
        return CompletableFuture.supplyAsync(this::validateAll, executorService);
    }

    /**
     * Repairs all components asynchronously.
     * @return CompletableFuture with repair result
     */
    public CompletableFuture<Boolean> repairAllAsync() {
        return CompletableFuture.supplyAsync(this::repairAll, executorService);
    }

    /**
     * Validates and repairs components if needed.
     * @return true if validation passed or repair succeeded
     */
    public boolean validateAndRepairIfNeeded() {
        if (validateAll()) {
            System.out.println("[RepairManager] All components are valid!");
            logManager.logInfo("All components validated successfully");
            return true;
        } else {
            System.out.println("[RepairManager] Invalid components detected, attempting repair...");
            logManager.logInfo("Invalid components detected, attempting repair");
            return repairAll() && validateAll();
        }
    }

    /**
     * Validates and repairs components asynchronously.
     * @return CompletableFuture with result
     */
    public CompletableFuture<Boolean> validateAndRepairIfNeededAsync() {
        return CompletableFuture.supplyAsync(this::validateAndRepairIfNeeded, executorService);
    }

    /**
     * Gets a report of all validation results.
     * @return Validation report
     */
    public RepairReport getValidationReport() {
        System.out.println("[RepairManager] Generating validation report...");
        RepairReport report = new RepairReport();

        for (Validator validator : validators) {
            try {
                boolean isValid = validator.validate();
                report.addValidatorResult(validator.getName(), isValid);
            } catch (Exception e) {
                report.addValidatorResult(validator.getName(), false);
                report.addError(validator.getName(), e.getMessage());
            }
        }

        return report;
    }

    /**
     * Prints a summary of validation results.
     */
    public void printValidationSummary() {
        RepairReport report = getValidationReport();
        System.out.println("\n" + "=".repeat(60));
        System.out.println("Repair Manager Validation Summary");
        System.out.println("=".repeat(60));

        for (Map.Entry<String, Boolean> entry : report.getResults().entrySet()) {
            String status = entry.getValue() ? "✓ VALID" : "✗ INVALID";
            System.out.println(entry.getKey() + ": " + status);
        }

        if (!report.getErrors().isEmpty()) {
            System.out.println("\nErrors:");
            for (Map.Entry<String, String> error : report.getErrors().entrySet()) {
                System.out.println("  " + error.getKey() + ": " + error.getValue());
            }
        }

        System.out.println("=".repeat(60) + "\n");
    }

    /**
     * Closes all resources.
     */
    public void close() {
        executorService.shutdown();
        try {
            if (!executorService.awaitTermination(10, TimeUnit.SECONDS)) {
                executorService.shutdownNow();
            }
        } catch (InterruptedException e) {
            executorService.shutdownNow();
        }
        System.out.println("[RepairManager] RepairManager closed");
    }

    /**
     * Gets the number of registered validators.
     * @return Validator count
     */
    public int getValidatorCount() {
        return validators.size();
    }

    /**
     * Report class for validation results.
     */
    public static class RepairReport {
        private final Map<String, Boolean> results = new LinkedHashMap<>();
        private final Map<String, String> errors = new HashMap<>();

        public void addValidatorResult(String name, boolean isValid) {
            results.put(name, isValid);
        }

        public void addError(String name, String error) {
            errors.put(name, error);
        }

        public Map<String, Boolean> getResults() {
            return results;
        }

        public Map<String, String> getErrors() {
            return errors;
        }

        public int getTotalValidators() {
            return results.size();
        }

        public int getValidCount() {
            return (int) results.values().stream().filter(v -> v).count();
        }

        public int getInvalidCount() {
            return getTotalValidators() - getValidCount();
        }

        public boolean isAllValid() {
            return getInvalidCount() == 0;
        }

        @Override
        public String toString() {
            return String.format("RepairReport{valid=%d/%d, errors=%d}", 
                    getValidCount(), getTotalValidators(), errors.size());
        }
    }
}
