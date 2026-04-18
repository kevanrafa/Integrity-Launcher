package com.integritylauncher.selfhealing;

import java.io.*;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.time.LocalDateTime;
import java.time.format.DateTimeFormatter;
import java.util.*;
import java.util.concurrent.*;

/**
 * Manages logging of Minecraft process output (stdout/stderr).
 * Captures logs, saves to file, and uploads to mclo.gs for sharing.
 */
public class LogManager {
    private static final String MCLO_GS_API = "https://mclo.gs/api/";
    private static final String LOG_FILENAME = "latest.log";
    private static final DateTimeFormatter TIMESTAMP_FORMAT = DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm:ss.SSS");

    private final Path logsDir;
    private final PrintWriter logWriter;
    private final int maxLogSizeBytes;
    private final ExecutorService executorService;

    public LogManager(Path logsDir, int maxLogSizeBytes) throws IOException {
        this.logsDir = logsDir;
        this.maxLogSizeBytes = maxLogSizeBytes;
        this.executorService = Executors.newSingleThreadExecutor(r -> {
            Thread t = new Thread(r, "LogManager-Uploader");
            t.setDaemon(true);
            return t;
        });

        // Create logs directory if needed
        Files.createDirectories(logsDir);

        // Initialize log file
        Path logFile = logsDir.resolve(LOG_FILENAME);
        this.logWriter = new PrintWriter(new FileWriter(logFile.toFile(), false), true);
        writeLogHeader();
    }

    /**
     * Writes header information to the log file.
     */
    private void writeLogHeader() {
        logWriter.println("=".repeat(80));
        logWriter.println("Integrity Launcher - Minecraft Log");
        logWriter.println("Started: " + LocalDateTime.now().format(TIMESTAMP_FORMAT));
        logWriter.println("Java Version: " + System.getProperty("java.version"));
        logWriter.println("OS: " + System.getProperty("os.name") + " " + System.getProperty("os.version"));
        logWriter.println("=".repeat(80));
        logWriter.flush();
    }

    /**
     * Captures and logs output from a Minecraft process.
     * @param process Minecraft process
     */
    public void captureProcessLogs(Process process) {
        System.out.println("[LogManager] Starting log capture for Minecraft process...");

        // Capture stdout
        new Thread(() -> captureStream(process.getInputStream(), "STDOUT")).start();

        // Capture stderr
        new Thread(() -> captureStream(process.getErrorStream(), "STDERR")).start();
    }

    /**
     * Captures a stream (stdout or stderr) and logs it.
     * @param stream Input stream to capture
     * @param streamType Type of stream (e.g., "STDOUT", "STDERR")
     */
    private void captureStream(InputStream stream, String streamType) {
        try (BufferedReader reader = new BufferedReader(new InputStreamReader(stream, StandardCharsets.UTF_8))) {
            String line;
            while ((line = reader.readLine()) != null) {
                logLine(streamType, line);
            }
        } catch (IOException e) {
            logError("Error reading from " + streamType + ": " + e.getMessage());
        }
    }

    /**
     * Logs a single line with timestamp and level.
     * @param level Log level or stream type
     * @param message Message to log
     */
    private void logLine(String level, String message) {
        String timestamp = LocalDateTime.now().format(TIMESTAMP_FORMAT);
        String formattedLine = String.format("[%s] [%s] %s", timestamp, level, message);

        logWriter.println(formattedLine);
        logWriter.flush();

        System.out.println(formattedLine);
    }

    /**
     * Logs an error message.
     * @param message Error message
     */
    public void logError(String message) {
        logLine("ERROR", message);
    }

    /**
     * Logs an info message.
     * @param message Info message
     */
    public void logInfo(String message) {
        logLine("INFO", message);
    }

    /**
     * Uploads the current log file to mclo.gs asynchronously.
     * @return CompletableFuture with the shareable log link
     */
    public CompletableFuture<String> uploadLogAsync() {
        return CompletableFuture.supplyAsync(() -> {
            try {
                return uploadLogToMclogs();
            } catch (Exception e) {
                System.out.println("[LogManager] Log upload failed: " + e.getMessage());
                return null;
            }
        }, executorService);
    }

    /**
     * Uploads the log file to mclo.gs and returns the shareable link.
     * @return mclo.gs shareable link (e.g., "https://mclo.gs/abcdef123")
     */
    public String uploadLogToMclogs() throws Exception {
        Path logFile = logsDir.resolve(LOG_FILENAME);

        if (!Files.exists(logFile)) {
            throw new Exception("Log file not found: " + logFile);
        }

        System.out.println("[LogManager] Uploading log to mclo.gs...");

        // Read log content
        byte[] logContent = Files.readAllBytes(logFile);

        // Create multipart form data
        String boundary = "----" + System.nanoTime();
        ByteArrayOutputStream body = new ByteArrayOutputStream();
        body.write(("--" + boundary + "\r\n").getBytes(StandardCharsets.UTF_8));
        body.write(("Content-Disposition: form-data; name=\"content\"\r\n\r\n").getBytes(StandardCharsets.UTF_8));
        body.write(logContent);
        body.write(("\r\n--" + boundary + "--\r\n").getBytes(StandardCharsets.UTF_8));

        byte[] bodyBytes = body.toByteArray();

        // Send request to mclo.gs
        URL url = new URL(MCLO_GS_API + "logs");
        HttpURLConnection conn = (HttpURLConnection) url.openConnection();
        conn.setRequestMethod("POST");
        conn.setDoOutput(true);
        conn.setConnectTimeout(10000);
        conn.setReadTimeout(10000);

        // Set headers
        conn.setRequestProperty("Content-Type", "multipart/form-data; boundary=" + boundary);
        conn.setRequestProperty("Content-Length", String.valueOf(bodyBytes.length));
        conn.setRequestProperty("User-Agent", "IntegrityLauncher/1.0");

        // Write body
        try (OutputStream os = conn.getOutputStream()) {
            os.write(bodyBytes);
            os.flush();
        }

        // Read response
        int responseCode = conn.getResponseCode();
        System.out.println("[LogManager] mclo.gs response code: " + responseCode);

        if (responseCode == HttpURLConnection.HTTP_OK) {
            BufferedReader reader = new BufferedReader(new InputStreamReader(conn.getInputStream(), StandardCharsets.UTF_8));
            String response = reader.readLine();
            reader.close();

            // Parse response (typically JSON with "id" field)
            String shareLink = parseShareLink(response);
            System.out.println("[LogManager] Log uploaded successfully: " + shareLink);
            return shareLink;
        } else {
            BufferedReader errorReader = new BufferedReader(new InputStreamReader(conn.getErrorStream(), StandardCharsets.UTF_8));
            String errorResponse = errorReader.readLine();
            errorReader.close();
            throw new Exception("Failed to upload log: HTTP " + responseCode + " - " + errorResponse);
        }
    }

    /**
     * Parses the mclo.gs API response to extract the share link.
     * @param response API response (JSON)
     * @return Share link URL
     */
    private String parseShareLink(String response) {
        // Simple JSON parsing for mclo.gs response
        // Response format: {"success":true,"id":"abc123"} or similar
        if (response.contains("\"id\"")) {
            int startIdx = response.indexOf("\"id\"") + 5;
            int endIdx = response.indexOf("\"", startIdx + 1);
            if (endIdx > startIdx) {
                String id = response.substring(startIdx + 1, endIdx);
                return "https://mclo.gs/" + id;
            }
        }

        // Fallback: try to extract any long string that looks like an ID
        String[] parts = response.split("[,{}\\[\\]\":]+");
        for (String part : parts) {
            if (part.length() > 4 && part.matches("[a-zA-Z0-9]+")) {
                return "https://mclo.gs/" + part;
            }
        }

        return "https://mclo.gs/";
    }

    /**
     * Gets the current log file path.
     * @return Path to latest.log
     */
    public Path getLogFile() {
        return logsDir.resolve(LOG_FILENAME);
    }

    /**
     * Saves the current log to a new file with a specific name.
     * @param filename Filename to save as
     */
    public void saveLogAs(String filename) throws IOException {
        Path source = getLogFile();
        Path destination = logsDir.resolve(filename);
        Files.copy(source, destination, StandardCopyOption.REPLACE_EXISTING);
        System.out.println("[LogManager] Log saved as: " + filename);
    }

    /**
     * Clears the current log file.
     */
    public void clearLog() throws IOException {
        Path logFile = getLogFile();
        logWriter.flush();
        logWriter.close();
        Files.delete(logFile);
        // Reinitialize log writer
        PrintWriter newWriter = new PrintWriter(new FileWriter(logFile.toFile(), false), true);
        System.out.println("[LogManager] Log file cleared");
    }

    /**
     * Closes all resources associated with the log manager.
     */
    public void close() {
        if (logWriter != null) {
            logWriter.flush();
            logWriter.close();
        }
        executorService.shutdown();
        try {
            if (!executorService.awaitTermination(5, TimeUnit.SECONDS)) {
                executorService.shutdownNow();
            }
        } catch (InterruptedException e) {
            executorService.shutdownNow();
        }
        System.out.println("[LogManager] LogManager closed");
    }

    /**
     * Gets the size of the current log file in bytes.
     * @return File size or -1 if not found
     */
    public long getLogFileSize() {
        try {
            return Files.size(getLogFile());
        } catch (IOException e) {
            return -1;
        }
    }

    /**
     * Checks if log file needs rotation (exceeds max size).
     * @return true if rotation needed
     */
    public boolean isRotationNeeded() {
        return getLogFileSize() > maxLogSizeBytes;
    }

    /**
     * Rotates the log file (saves current, creates new).
     */
    public void rotateLog() throws IOException {
        String timestamp = LocalDateTime.now().format(DateTimeFormatter.ofPattern("yyyy-MM-dd_HH-mm-ss"));
        saveLogAs("log_" + timestamp + ".log");
        System.out.println("[LogManager] Log rotated: log_" + timestamp + ".log");
        logWriter.flush();
    }
}
