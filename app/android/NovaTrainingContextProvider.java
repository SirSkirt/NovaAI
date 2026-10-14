package com.qtech.neuralnova.experimental;

import android.app.Notification;
import android.app.NotificationManager;
import android.content.SharedPreferences;
import android.service.notification.StatusBarNotification;
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
 * Supplies a legitimate Android application Context to the foreground
 * lifecycle adapter, including when there is no Activity or live WebView.
 * No neural computation, files, or conversation data are handled here.
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
     * Update the SAME foreground-service notification, not a third item.
     * Invoked directly by Nova's native Rust ZIP scheduler at each lesson
     * transition. Does not restart, stop, or alter the running service.
     * LifecycleService uses SharedPreferences("bg_service") to configure
     * the channel and notification ID; mirror those exact settings.
     */
    public static boolean updateTrainingLesson(String lesson, int index, int total) {
        Context context = applicationContext;
        if (context == null || lesson == null || index < 1 || total < index) {
            return false;
        }
        SharedPreferences preferences = context.getSharedPreferences(
            "bg_service", Context.MODE_PRIVATE);
        int notificationId = preferences.getInt("bg_notif_id", 9001);
        // Update the actual active foreground notification rather than
        // constructing a new one. Recovering its builder preserves the icon,
        // channel, title, launch action, Android FGS flags and accessibility.
        NotificationManager manager = (NotificationManager) context.getSystemService(
            Context.NOTIFICATION_SERVICE);
        if (manager == null) return false;
        Notification original = null;
        for (StatusBarNotification posted : manager.getActiveNotifications()) {
            if (posted.getId() == notificationId && posted.getTag() == null) {
                original = posted.getNotification();
                break;
            }
        }
        if (original == null) return false; // service is not running; do not resurrect it.
        String detail = "Lesson " + index + "/" + total + ": " + lesson;
        Notification.Builder builder = Notification.Builder.recoverBuilder(
            context, original);
        builder.setContentText(detail)
            .setStyle(new Notification.BigTextStyle().bigText(detail))
            .setOnlyAlertOnce(true);
        Notification notification = builder.build();
        try {
            manager.notify(notificationId, notification);
            return true;
        } catch (RuntimeException error) {
            android.util.Log.w("NovaTrainingNotification",
                "Foreground lesson notification update failed", error);
            return false;
        }
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
                    !source.getName().endsWith(".nhyb") || !source.isFile() ||
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
