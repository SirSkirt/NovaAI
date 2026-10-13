package com.qtech.neuralnova.experimental;

import android.app.Notification;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.content.Intent;
import android.content.SharedPreferences;
import android.os.Build;
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
        String channelId = preferences.getString("bg_notif_channel_id", "bg_keepalive");
        if (channelId == null || channelId.isEmpty()) channelId = "bg_keepalive";

        // Never resurrect a stopped foreground-service notification.
        // The Kotlin LifecycleService sets isForeground=true after promotion.
        if (!app.tauri.backgroundservice.LifecycleService.Companion.isForeground()) {
            return false;
        }
        NotificationManager manager = (NotificationManager) context.getSystemService(
            Context.NOTIFICATION_SERVICE);
        if (manager == null) return false;
        Intent launch = context.getPackageManager().getLaunchIntentForPackage(
            context.getPackageName());
        PendingIntent pending = launch == null ? null : PendingIntent.getActivity(
            context, 0, launch,
            PendingIntent.FLAG_IMMUTABLE | PendingIntent.FLAG_UPDATE_CURRENT);
        String detail = "Lesson " + index + "/" + total + ": " + lesson;
        Notification.Builder builder = new Notification.Builder(context)
            .setSmallIcon(context.getApplicationInfo().icon)
            .setContentTitle(context.getApplicationInfo()
                .loadLabel(context.getPackageManager()))
            .setContentText(detail)
            .setStyle(new Notification.BigTextStyle().bigText(detail))
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .setSound(null)
            .setDefaults(0)
            .setPriority(Notification.PRIORITY_LOW)
            .setCategory(Notification.CATEGORY_SERVICE);
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            builder.setChannelId(channelId);
        }
        if (pending != null) builder.setContentIntent(pending);
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
