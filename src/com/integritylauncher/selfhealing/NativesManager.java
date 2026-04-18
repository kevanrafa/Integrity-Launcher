package com.integritylauncher.selfhealing;

import java.io.*;
import java.nio.file.*;
import java.util.*;
import java.util.jar.JarEntry;
import java.util.jar.JarFile;
import java.util.zip.ZipEntry;
import java.util.zip.ZipFile;

/**
 * Manages extraction, validation, and repair of native libraries (LWJGL, etc.).
 * Handles platform-specific native files (.dll, .so, .dylib).
 */
public class NativesManager implements Validator {
    private static final String[] WINDOWS_EXTENSIONS = {".dll"};
    private static final String[] LINUX_EXTENSIONS = {".so"};
    private static final String[] MACOS_EXTENSIONS = {".dylib", ".jnilib"};

    private final Path nativesDir;
    private final Path librariesDir;
    private final Path tempDir;
    private final String osName;
    private final int maxRetries;

    public NativesManager(Path nativesDir, Path librariesDir, Path tempDir, int maxRetries) {
        this.nativesDir = nativesDir;
        this.librariesDir = librariesDir;
        this.tempDir = tempDir;
        this.maxRetries = maxRetries;
        this.osName = System.getProperty("os.name").toLowerCase();
    }

    @Override
    public String getName() {
        return "NativesManager";
    }

    @Override
    public boolean validate() throws Exception {
        // Check if nativesDir exists and contains required native files
        if (!Files.exists(nativesDir)) {
            System.out.println("[NativesManager] Natives directory does not exist: " + nativesDir);
            return false;
        }

        String[] requiredExtensions = getRequiredExtensions();
        boolean hasNatives = false;

        try (DirectoryStream<Path> stream = Files.newDirectoryStream(nativesDir)) {
            for (Path file : stream) {
                String fileName = file.getFileName().toString().toLowerCase();
                for (String ext : requiredExtensions) {
                    if (fileName.endsWith(ext)) {
                        hasNatives = true;
                        System.out.println("[NativesManager] Found native file: " + file);
                    }
                }
            }
        }

        if (!hasNatives) {
            System.out.println("[NativesManager] No native files found in " + nativesDir);
            return false;
        }

        return validateNativeFiles();
    }

    @Override
    public void repair() throws Exception {
        System.out.println("[NativesManager] Starting native repair...");
        Files.createDirectories(nativesDir);

        int attempt = 0;
        while (attempt < maxRetries) {
            try {
                extractAllNatives();
                if (validate()) {
                    System.out.println("[NativesManager] Natives repair successful!");
                    return;
                }
            } catch (Exception e) {
                System.out.println("[NativesManager] Extraction attempt " + (attempt + 1) + " failed: " + e.getMessage());
            }
            attempt++;
        }

        throw new Exception("Failed to repair natives after " + maxRetries + " attempts");
    }

    /**
     * Extracts all native libraries from JAR files in the libraries directory.
     */
    private void extractAllNatives() throws Exception {
        System.out.println("[NativesManager] Extracting natives from library JARs...");

        try (DirectoryStream<Path> stream = Files.newDirectoryStream(librariesDir)) {
            for (Path file : stream) {
                if (file.toString().endsWith(".jar")) {
                    extractNativesFromJar(file);
                }
            }
        }
    }

    /**
     * Extracts native files from a single JAR file.
     */
    private void extractNativesFromJar(Path jarPath) throws Exception {
        try (JarFile jar = new JarFile(jarPath.toFile())) {
            Enumeration<JarEntry> entries = jar.entries();

            while (entries.hasMoreElements()) {
                JarEntry entry = entries.nextElement();
                if (isNativeFile(entry.getName())) {
                    extractNativeEntry(jar, entry);
                }
            }
        }
    }

    /**
     * Checks if a JAR entry is a native file for the current OS.
     */
    private boolean isNativeFile(String entryName) {
        String lower = entryName.toLowerCase();
        String[] extensions = getRequiredExtensions();

        for (String ext : extensions) {
            if (lower.endsWith(ext)) {
                return true;
            }
        }
        return false;
    }

    /**
     * Extracts a single native file entry from a JAR.
     */
    private void extractNativeEntry(JarFile jar, JarEntry entry) throws Exception {
        String fileName = new File(entry.getName()).getName();
        Path targetPath = nativesDir.resolve(fileName);

        Files.createDirectories(targetPath.getParent());

        try (InputStream in = jar.getInputStream(entry);
             OutputStream out = Files.newOutputStream(targetPath)) {
            byte[] buffer = new byte[8192];
            int bytesRead;
            while ((bytesRead = in.read(buffer)) != -1) {
                out.write(buffer, 0, bytesRead);
            }
        }

        System.out.println("[NativesManager] Extracted native: " + targetPath);
    }

    /**
     * Validates that extracted native files are not empty and are readable.
     */
    private boolean validateNativeFiles() throws Exception {
        try (DirectoryStream<Path> stream = Files.newDirectoryStream(nativesDir)) {
            for (Path file : stream) {
                if (isNativeFile(file.getFileName().toString())) {
                    long size = Files.size(file);
                    if (size == 0) {
                        System.out.println("[NativesManager] Native file is empty: " + file);
                        return false;
                    }
                    if (!Files.isReadable(file)) {
                        System.out.println("[NativesManager] Native file is not readable: " + file);
                        return false;
                    }
                }
            }
        }
        return true;
    }

    /**
     * Gets the required native file extensions for the current OS.
     */
    private String[] getRequiredExtensions() {
        if (osName.contains("win")) {
            return WINDOWS_EXTENSIONS;
        } else if (osName.contains("nix") || osName.contains("nux")) {
            return LINUX_EXTENSIONS;
        } else if (osName.contains("mac")) {
            return MACOS_EXTENSIONS;
        }
        return new String[]{};
    }

    /**
     * Gets the current operating system identifier.
     */
    public String getOsIdentifier() {
        if (osName.contains("win")) {
            return "windows";
        } else if (osName.contains("nix") || osName.contains("nux")) {
            return "linux";
        } else if (osName.contains("mac")) {
            return "macos";
        }
        return "unknown";
    }

    /**
     * Clears the natives directory and re-extracts from scratch.
     */
    public void cleanAndRepair() throws Exception {
        System.out.println("[NativesManager] Performing clean repair...");
        if (Files.exists(nativesDir)) {
            Files.walk(nativesDir)
                    .sorted(Comparator.reverseOrder())
                    .forEach(path -> {
                        try {
                            Files.delete(path);
                        } catch (IOException e) {
                            System.out.println("[NativesManager] Failed to delete: " + path);
                        }
                    });
        }
        repair();
    }
}
