package com.integritylauncher.selfhealing;

/**
 * Interface for validating and repairing game components.
 */
public interface Validator {
    /**
     * Validates if a component is in a valid state.
     * @return true if valid, false if invalid or missing
     * @throws Exception if validation fails
     */
    boolean validate() throws Exception;

    /**
     * Repairs a component if it's invalid or missing.
     * @throws Exception if repair fails
     */
    void repair() throws Exception;

    /**
     * Gets the name of this validator for logging purposes.
     * @return validator name
     */
    String getName();
}
