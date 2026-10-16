package com.qtech.neuralnova.experimental;

import android.content.ContentProvider;
import android.content.ContentValues;
import android.content.Context;
import android.database.Cursor;
import android.net.Uri;
import java.io.File;
import java.io.FileInputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.security.MessageDigest;
import java.util.Arrays;

/**
 * Provides the application-private Android Context required for reliable
 * Storage Access Framework checkpoint export. No training service is used.
 * Not exported: no other app can access this provider.
 */
public final class NovaTrainingContextProvider extends ContentProvider {
    private static volatile Context applicationContext;

    @Override public boolean onCreate() {
        Context current = getContext();
        if (current != null) applicationContext = current.getApplicationContext();
        return applicationContext != null;
    }

    public static Context getAppContext() {
        return applicationContext;
    }

    /**
     * Android Storage Access Framework export.
     *
     * The system's ACTION_CREATE_DOCUMENT dialog grants access to a
     * content:// URI. The Tauri fs plugin can detach a ParcelFileDescriptor
     * too early, so some providers publish a 0-byte file even after a
     * seemingly successful write. Keep the ContentResolver output stream
     * open in Java until ALL bytes are written and the provider has seen
     * close(), then independently reopen and verify the exact SHA-256.
     *
     * App-private ONLY; no public directory access, broad storage permission,
     * external command execution, or neural inference takes place here.
     */
    public static String writeVerifiedCheckpoint(String sourcePath, String destinationUri) {
        Context context = applicationContext;
        if (context == null) return "ERROR:Android application context unavailable";
        if (sourcePath == null || destinationUri == null ||
                !destinationUri.startsWith("content://")) {
            return "ERROR:Select a destination using Android Files";
        }
        try {
            File source = new File(sourcePath).getCanonicalFile();
            File privateRoot = context.getDataDir().getCanonicalFile();
            if (!source.getPath().startsWith(privateRoot.getPath() + File.separator) ||
                    !source.getName().endsWith(".nnova2") || !source.isFile() ||
                    source.length() < 1 || source.length() > 24L * 1024L * 1024L) {
                return "ERROR:Refusing export from an invalid or non-private checkpoint file";
            }
            Uri uri = Uri.parse(destinationUri);
            MessageDigest expected = MessageDigest.getInstance("SHA-256");
            long written = 0;
            try (FileInputStream input = new FileInputStream(source);
                 OutputStream output = context.getContentResolver().openOutputStream(uri, "wt")) {
                if (output == null) return "ERROR:Android Files could not open the destination";
                byte[] buffer = new byte[65536];
                int count;
                while ((count = input.read(buffer)) != -1) {
                    output.write(buffer, 0, count);
                    expected.update(buffer, 0, count);
                    written += count;
                }
                output.flush();
            } // CLOSE THE NATIVE STREAM before checking metadata or contents.
            if (written != source.length()) return "ERROR:Incomplete checkpoint document write";
            MessageDigest actual = MessageDigest.getInstance("SHA-256");
            long verified = 0;
            try (InputStream input = context.getContentResolver().openInputStream(uri)) {
                if (input == null) return "ERROR:Android Files could not reopen saved checkpoint";
                byte[] buffer = new byte[65536];
                int count;
                while ((count = input.read(buffer)) != -1) {
                    actual.update(buffer, 0, count);
                    verified += count;
                    if (verified > written) break;
                }
            }
            if (written != verified || !Arrays.equals(expected.digest(), actual.digest())) {
                return "ERROR:Saved document failed SHA-256 read-back verification (" +
                        verified + "/" + written + " bytes); do not rely on this backup";
            }
            return "OK:" + verified;
        } catch (Exception problem) {
            android.util.Log.e("NovaCheckpointExport", "Android SAF checkpoint save failed", problem);
            return "ERROR:" + problem.getMessage();
        }
    }

    @Override public Cursor query(Uri uri, String[] projection, String selection,
                                  String[] selectionArgs, String sortOrder) {
        return null;
    }
    @Override public String getType(Uri uri) { return null; }
    @Override public Uri insert(Uri uri, ContentValues values) { return null; }
    @Override public int delete(Uri uri, String selection, String[] selectionArgs) { return 0; }
    @Override public int update(Uri uri, ContentValues values, String selection,
                                String[] selectionArgs) { return 0; }
}
