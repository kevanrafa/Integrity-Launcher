package com.integritylauncher.selfhealing;

import java.io.*;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.file.*;
import java.security.MessageDigest;
import java.util.*;

/**
 * Manages validation and repair of Minecraft libraries, version JARs, and assets.
 */
public class LibraryManager implements Validator {
    private final Path librariesDir;
    private final Path assetsDir;
    private final Path versionJarsDir;
    private final int maxRetries;
    private final int connectionTimeout;

    public LibraryManager(Path librariesDir, Path assetsDir, Path versionJarsDir, int maxRetries, int connectionTimeout) {
        this.librariesDir = librariesDir;
        this.assetsDir = assetsDir;
        this.versionJarsDir = versionJarsDir;
        this.maxRetries = maxRetries;
        this.connectionTimeout = connectionTimeout;
    }

    @Override
    public String getName() {
        return "LibraryManager";
    }

    @Override
    public boolean validate() throws Exception {
        System.out.println("[LibraryManager] Validating libraries...");

        // Check if required directories exist
        if (!Files.exists(librariesDir)) {
            System.out.println("[LibraryManager] Libraries directory does not exist: " + librariesDir);
            return false;
        }

        if (!Files.exists(assetsDir)) {
            System.out.println("[LibraryManager] Assets directory does not exist: " + assetsDir);
            return false;
        }

        // Check for empty or corrupted files
        if (!validateDirectoryContents(librariesDir)) {
            System.out.println("[LibraryManager] Found corrupted or empty library files");
            return false;
        }

        if (!validateDirectoryContents(assetsDir)) {
            System.out.println("[LibraryManager] Found corrupted or empty asset files");
            return false;
        }

        System.out.println("[LibraryManager] All libraries validated successfully");
        return true;
    }

    @Override
    public void repair() throws Exception {
        System.out.println("[LibraryManager] Starting library repair...");

        Files.createDirectories(librariesDir);
        Files.createDirectories(assetsDir);
        Files.createDirectories(versionJarsDir);

        repairCorruptedFiles(librariesDir);
        repairCorruptedFiles(assetsDir);

        System.out.println("[LibraryManager] Library repair completed");
    }

    /**
     * Validates that all files in a directory are not empty.
     * @param directory Directory to validate
     * @return true if all files are valid
     */
    private boolean validateDirectoryContents(Path directory) throws IOException {
        try (DirectoryStream<Path> stream = Files.newDirectoryStream(directory)) {
            for (Path file : stream) {
                if (Files.isRegularFile(file)) {
                    if (Files.size(file) == 0) {
                        System.out.println("[LibraryManager] Empty file found: " + file);
                        return false;
                    }
                } else if (Files.isDirectory(file)) {
                    if (!validateDirectoryContents(file)) {
                        return false;
                    }
                }
            }
        }
        return true;
    }

    /**
     * Repairs corrupted files in a directory by removing them.
     * @param directory Directory to repair
     */
    private void repairCorruptedFiles(Path directory) throws IOException {
        try (DirectoryStream<Path> stream = Files.newDirectoryStream(directory)) {
            for (Path file : stream) {
                if (Files.isRegularFile(file)) {
                    if (Files.size(file) == 0) {
                        System.out.println("[LibraryManager] Removing corrupted file: " + file);
                        Files.delete(file);
                    }
                } else if (Files.isDirectory(file)) {
                    repairCorruptedFiles(file);
                }
            }
        }
    }

    /**
     * Downloads a library from a given URL to a local path.
     * @param downloadUrl URL to download from
     * @param targetPath Local path to save to
     * @param expectedSha1 Expected SHA-1 hash (optional, null to skip)
     */
    public void downloadLibrary(String downloadUrl, Path targetPath, String expectedSha1) throws Exception {
        Files.createDirectories(targetPath.getParent());

        int attempt = 0;
        while (attempt < maxRetries) {
            try {
                System.out.println("[LibraryManager] Downloading library (attempt " + (attempt + 1) + "): " + downloadUrl);

                URL url = new URL(downloadUrl);
                HttpURLConnection conn = (HttpURLConnection) url.openConnection();
                conn.setConnectTimeout(connectionTimeout);
                conn.setReadTimeout(connectionTimeout);
                conn.setRequestProperty("User-Agent", "IntegrityLauncher/1.0");

                int responseCode = conn.getResponseCode();
                if (responseCode != HttpURLConnection.HTTP_OK) {
                    throw new Exception("HTTP " + responseCode + " for " + downloadUrl);
                }

                // Download file
                try (InputStream in = conn.getInputStream();
                     OutputStream out = Files.newOutputStream(targetPath)) {
                    byte[] buffer = new byte[8192];
                    int bytesRead;
                    while ((bytesRead = in.read(buffer)) != -1) {
                        out.write(buffer, 0, bytesRead);
                    }
                }

                // Verify hash if provided
                if (expectedSha1 != null && !expectedSha1.isEmpty()) {
                    String actualSha1 = calculateSha1(targetPath);
                    if (!actualSha1.equalsIgnoreCase(expectedSha1)) {
                        System.out.println("[LibraryManager] SHA-1 mismatch for " + targetPath);
                        System.out.println("[LibraryManager] Expected: " + expectedSha1 + ", Got: " + actualSha1);
                        Files.delete(targetPath);
                        throw new Exception("SHA-1 verification failed");
                    }
                }

                System.out.println("[LibraryManager] Successfully downloaded: " + targetPath);
                return;

            } catch (Exception e) {
                System.out.println("[LibraryManager] Download attempt " + (attempt + 1) + " failed: " + e.getMessage());
                attempt++;

                if (Files.exists(targetPath)) {
                    try {
                        Files.delete(targetPath);
                    } catch (IOException ignored) {}
                }
            }
        }

        throw new Exception("Failed to download library after " + maxRetries + " attempts: " + downloadUrl);
    }

    /**
     * Calculates the SHA-1 hash of a file.
     * @param file File to hash
     * @return Hex string of SHA-1 hash
     */
    public String calculateSha1(Path file) throws Exception {
        MessageDigest digest = MessageDigest.getInstance("SHA-1");
        byte[] buffer = new byte[8192];
        int bytesRead;

        try (InputStream in = Files.newInputStream(file)) {
            while ((bytesRead = in.read(buffer)) != -1) {
                digest.update(buffer, 0, bytesRead);
            }
        }

        byte[] hashBytes = digest.digest();
        StringBuilder sb = new StringBuilder();
        for (byte b : hashBytes) {
            sb.append(String.format("%02x", b));
        }
        return sb.toString();
    }

    /**
     * Validates that a library file matches its expected SHA-1 hash.
     * @param filePath Path to library file
     * @param expectedSha1 Expected SHA-1 hash
     * @return true if hash matches
     */
    public boolean validateLibrarySha1(Path filePath, String expectedSha1) throws Exception {
        if (!Files.exists(filePath)) {
            return false;
        }

        String actualSha1 = calculateSha1(filePath);
        return actualSha1.equalsIgnoreCase(expectedSha1);
    }

    /**
     * Gets all JAR files in the libraries directory.
     * @return List of JAR file paths
     */
    public List<Path> getAllLibraryJars() throws IOException {
        List<Path> jars = new ArrayList<>();
        Files.walk(librariesDir)
                .filter(p -> p.toString().endsWith(".jar"))
                .forEach(jars::add);
        return jars;
    }

    /**
     * Gets all asset files in the assets directory.
     * @return List of asset file paths
     */
    public List<Path> getAllAssets() throws IOException {
        List<Path> assets = new ArrayList<>();
        Files.walk(assetsDir)
                .filter(Files::isRegularFile)
                .forEach(assets::add);
        return assets;
    }

    /**
     * Checks if a specific library exists and is valid.
     * @param libraryPath Relative path to library
     * @return true if library exists and is not empty
     */
    public boolean libraryExists(String libraryPath) throws IOException {
        Path fullPath = librariesDir.resolve(libraryPath);
        return Files.exists(fullPath) && Files.size(fullPath) > 0;
    }

    /**
     * Removes all corrupted or empty files recursively.
     * @param directory Directory to clean
     */
    public void cleanCorruptedFiles(Path directory) throws IOException {
        Files.walk(directory)
                .sorted(Comparator.reverseOrder())
                .forEach(path -> {
                    try {
                        if (Files.isRegularFile(path) && Files.size(path) == 0) {
                            System.out.println("[LibraryManager] Removing corrupted file: " + path);
                            Files.delete(path);
                        }
                    } catch (IOException e) {
                        System.out.println("[LibraryManager] Failed to remove: " + path);
                    }
                });
    }

    /**
     * Gets statistics about the library directory.
     * @return Statistics string
     */
    public String getLibraryStatistics() throws IOException {
        List<Path> allJars = getAllLibraryJars();
        long totalSize = 0;
        int emptyCount = 0;

        for (Path jar : allJars) {
            long size = Files.size(jar);
            totalSize += size;
            if (size == 0) emptyCount++;
        }

        return String.format("Total JARs: %d, Empty: %d, Total Size: %.2f MB", 
                allJars.size(), emptyCount, totalSize / (1024.0 * 1024.0));
    }
}
