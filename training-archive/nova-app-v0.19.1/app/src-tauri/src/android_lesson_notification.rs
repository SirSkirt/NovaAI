//! Foreground-service notification text for the EXISTING Android service.
//! This is presentation only. It does not run or affect Nova's neural model.
//! Calling this from the Rust ZIP worker works when Android suspends WebView JS.
use jni::objects::{JClass,JObject,JValue};
use jni::JavaVM;

/// Update notification ID 9001 used by the upstream background-service
/// plugin. The app-owned, nonexported Android provider builds a replacement
/// notification with the same ID/channel and an expanded lesson filename.
pub(crate) fn show_zip_lesson(lesson:&str,index:usize,total:usize)->bool {
    if index==0 || total<index {return false;}
    match try_show_zip_lesson(lesson,index,total){
        Ok(())=>true,
        Err(error)=>{
            // A missing foreground notification can occur during Android's
            // initial service promotion. The keepalive retries after 8 sec.
            eprintln!("Neural Nova: unable to refresh foreground lesson text: {error}");
            false
        }
    }
}
fn try_show_zip_lesson(lesson:&str,index:usize,total:usize)->Result<(),String>{
    let context=ndk_context::android_context();
    // SAFETY: Tauri initialized this process's Android JavaVM before native
    // commands run. ndk-context stores the actual process JavaVM pointer.
    let vm=unsafe{JavaVM::from_raw(context.vm() as *mut jni::sys::JavaVM)}
        .map_err(|e|format!("Android JavaVM unavailable: {e}"))?;
    let mut env=vm.attach_current_thread()
        .map_err(|e|format!("Failed to attach Rust ZIP worker to Android VM: {e}"))?;
    // A worker thread attached via JavaVM may not inherit the app's class
    // loader. Resolve the app-owned provider through the real Android Context.
    // ndk-context holds that Context's global JNI reference; DO NOT drop it as
    // a Rust-owned local reference.
    let app_context=std::mem::ManuallyDrop::new(unsafe {
        JObject::from_raw(context.context() as jni::sys::jobject)
    });
    let loader=env.call_method(
        &*app_context,"getClassLoader","()Ljava/lang/ClassLoader;",&[]
    ).map_err(|e|format!("Cannot obtain app class loader: {e}"))?
        .l().map_err(|e|e.to_string())?;
    let class_name=env.new_string(
        "com.qtech.neuralnova.experimental.NovaTrainingContextProvider"
    ).map_err(|e|e.to_string())?;
    let notification_class=env.call_method(
        loader,"loadClass","(Ljava/lang/String;)Ljava/lang/Class;",
        &[JValue::Object(class_name.as_ref())]
    ).map_err(|e|format!("Cannot load Nova notification provider: {e}"))?
        .l().map_err(|e|e.to_string())?;
    let notification_class=JClass::from(notification_class);
    let name=env.new_string(lesson).map_err(|e|format!("Invalid lesson label: {e}"))?;
    let accepted=env.call_static_method(
        notification_class,
        "updateTrainingLesson",
        "(Ljava/lang/String;II)Z",
        &[
            JValue::Object(name.as_ref()),
            JValue::Int(i32::try_from(index).map_err(|e|e.to_string())?),
            JValue::Int(i32::try_from(total).map_err(|e|e.to_string())?),
        ],
    ).map_err(|e|format!("Foreground notification JNI call failed: {e}"))?
        .z().map_err(|e|format!("Foreground notification return type invalid: {e}"))?;
    if !accepted{return Err("Android foreground service isn't ready to update notification yet".into());}
    Ok(())
}
