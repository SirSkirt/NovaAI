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
