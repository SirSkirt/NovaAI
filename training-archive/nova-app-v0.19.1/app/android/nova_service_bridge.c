/*
 * Neural Nova foreground-service lifecycle adapter, NOT a neural model.
 *
 * tauri-plugin-background-service 1.0.1 unconditionally calls JNI
 * HeadlessBridge.startCore() from LifecycleService, even for apps that use
 * only its foreground keepalive. The default "app_core" library is an
 * optional headless application-specific core that Neural Nova does not ship.
 *
 * This deliberately tiny library acknowledges start/stop and holds a
 * PARTIAL_WAKE_LOCK while teaching. The real neural computation stays in
 * Neural Nova's EXISTING Rust Tauri worker, in the SAME Android process.
 * There is no model, inference, answer selection, persistence or trainer here.
 */
#include <jni.h>
#include <pthread.h>
#include <string.h>

static pthread_mutex_t lock = PTHREAD_MUTEX_INITIALIZER;
static jobject wake_lock = NULL;

static void clear_java_exception(JNIEnv *env) {
    if ((*env)->ExceptionCheck(env)) (*env)->ExceptionClear(env);
}

static jstring result(JNIEnv *env, const char *state) {
    return (*env)->NewStringUTF(env, state);
}

static void release_wake_lock(JNIEnv *env) {
    pthread_mutex_lock(&lock);
    if (wake_lock != NULL) {
        jclass cls = (*env)->GetObjectClass(env, wake_lock);
        if (cls != NULL) {
            jmethodID release = (*env)->GetMethodID(env, cls, "release", "()V");
            if (release != NULL) (*env)->CallVoidMethod(env, wake_lock, release);
            clear_java_exception(env);
            (*env)->DeleteLocalRef(env, cls);
        }
        (*env)->DeleteGlobalRef(env, wake_lock);
        wake_lock = NULL;
    }
    pthread_mutex_unlock(&lock);
}

static int acquire_wake_lock(JNIEnv *env) {
    pthread_mutex_lock(&lock);
    if (wake_lock != NULL) { pthread_mutex_unlock(&lock); return 1; }

    // Primary route: app-owned ContentProvider with an ordinary public
    // Android Context. This works on Android 16 without restricted SDK calls.
    jobject app = NULL;
    jclass provider = (*env)->FindClass(env,
        "com/qtech/neuralnova/experimental/NovaTrainingContextProvider");
    if (provider != NULL) {
        jmethodID get_app = (*env)->GetStaticMethodID(env, provider,
            "getAppContext", "()Landroid/content/Context;");
        if (get_app != NULL)
            app = (*env)->CallStaticObjectMethod(env, provider, get_app);
    }
    if ((*env)->ExceptionCheck(env)) {
        clear_java_exception(env);
        app = NULL;
    }
    if (app == NULL) {
        // Compatibility fallback if an older generated Android project
        // omitted the provider. Never silently proceed without a wake lock.
        jclass thread = (*env)->FindClass(env, "android/app/ActivityThread");
        if (thread == NULL) goto fail;
        jmethodID app_method = (*env)->GetStaticMethodID(env, thread,
            "currentApplication", "()Landroid/app/Application;");
        if (app_method == NULL) goto fail;
        app = (*env)->CallStaticObjectMethod(env, thread, app_method);
        if (app == NULL || (*env)->ExceptionCheck(env)) goto fail;
    }

    jclass context = (*env)->FindClass(env, "android/content/Context");
    if (context == NULL) goto fail;
    jmethodID service_method = (*env)->GetMethodID(env, context,
        "getSystemService", "(Ljava/lang/String;)Ljava/lang/Object;");
    if (service_method == NULL) goto fail;
    jstring power_key = (*env)->NewStringUTF(env, "power");
    jobject power = (*env)->CallObjectMethod(env, app, service_method, power_key);
    if (power == NULL || (*env)->ExceptionCheck(env)) goto fail;

    jclass power_cls = (*env)->FindClass(env, "android/os/PowerManager");
    if (power_cls == NULL) goto fail;
    jmethodID new_lock = (*env)->GetMethodID(env, power_cls, "newWakeLock",
        "(ILjava/lang/String;)Landroid/os/PowerManager$WakeLock;");
    if (new_lock == NULL) goto fail;
    jstring tag = (*env)->NewStringUTF(env, "NeuralNova:Training");
    // PARTIAL_WAKE_LOCK = 1, not FULL_WAKE_LOCK; the screen can remain off.
    jobject candidate = (*env)->CallObjectMethod(env, power, new_lock, 1, tag);
    if (candidate == NULL || (*env)->ExceptionCheck(env)) goto fail;
    jclass wake_cls = (*env)->GetObjectClass(env, candidate);
    if (wake_cls == NULL) goto fail;
    jmethodID acquire = (*env)->GetMethodID(env, wake_cls, "acquire", "()V");
    if (acquire == NULL) goto fail;
    (*env)->CallVoidMethod(env, candidate, acquire);
    if ((*env)->ExceptionCheck(env)) goto fail;
    wake_lock = (*env)->NewGlobalRef(env, candidate);
    if (wake_lock == NULL) goto fail;
    pthread_mutex_unlock(&lock);
    return 1;
fail:
    clear_java_exception(env);
    pthread_mutex_unlock(&lock);
    return 0;
}

/* The static Kotlin @JvmStatic native declarations have these JNI symbols. */
JNIEXPORT jstring JNICALL
Java_app_tauri_backgroundservice_HeadlessBridge_startCore(
    JNIEnv *env, jclass cls, jstring data_dir, jstring reason) {
    (void)cls; (void)data_dir;
    // The Rust education session cannot be reconstructed by the plugin's
    // generic boot auto-restart. Never hold a CPU wake lock after an OS
    // restart without an explicit user-triggered native training session.
    if (reason == NULL)
        return result(env, "{\"ok\":false,\"state\":\"failed\",\"code\":\"resume_training_in_app\",\"recoverable\":true}");
    const char *why = (*env)->GetStringUTFChars(env, reason, NULL);
    if (why == NULL) return result(env, "{\"ok\":false,\"state\":\"failed\",\"code\":\"invalid_start_reason\"}");
    int manual = strcmp(why, "manual_start") == 0;
    (*env)->ReleaseStringUTFChars(env, reason, why);
    if (!manual)
        return result(env, "{\"ok\":false,\"state\":\"failed\",\"code\":\"resume_training_in_app\",\"recoverable\":true}");
    if (!acquire_wake_lock(env))
        return result(env, "{\"ok\":false,\"state\":\"failed\",\"code\":\"nova_wakelock_failed\",\"message\":\"Cannot acquire Android CPU wake lock for training\",\"recoverable\":true}");
    return result(env, "{\"ok\":true,\"state\":\"running\",\"recoverable\":false}");
}
JNIEXPORT jstring JNICALL
Java_app_tauri_backgroundservice_HeadlessBridge_stopCore(
    JNIEnv *env, jclass cls, jstring data_dir, jstring reason) {
    (void)cls; (void)data_dir; (void)reason;
    release_wake_lock(env);
    return result(env, "{\"ok\":true,\"state\":\"stopped\"}");
}
JNIEXPORT jstring JNICALL
Java_app_tauri_backgroundservice_HeadlessBridge_notifyNetworkChanged(
    JNIEnv *env, jclass cls) {
    (void)cls;
    return result(env, "{\"ok\":true,\"state\":\"running\"}");
}
/* Unused plugin call/message features have NO corresponding Nova actions. */
JNIEXPORT jstring JNICALL
Java_app_tauri_backgroundservice_HeadlessBridge_callAction(
    JNIEnv *env, jclass cls, jstring id, jstring action) {
    (void)cls; (void)id; (void)action;
    return result(env, "{\"ok\":false,\"state\":\"unsupported\",\"message\":\"Nova has no native calls\"}");
}
JNIEXPORT jstring JNICALL
Java_app_tauri_backgroundservice_HeadlessBridge_notificationAction(
    JNIEnv *env, jclass cls, jstring dir, jstring action, jstring chat,
    jstring message, jstring reply) {
    (void)cls; (void)dir; (void)action; (void)chat; (void)message; (void)reply;
    return result(env, "{\"ok\":false,\"state\":\"unsupported\",\"message\":\"Nova has no headless messaging\"}");
}
