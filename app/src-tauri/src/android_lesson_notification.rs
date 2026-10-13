//! Foreground-service notification text for the EXISTING Android service.
//! This is presentation only. It does not run or affect Nova's neural model.
//! Calling this from the Rust ZIP worker works when Android suspends WebView JS.
use jni::objects::JValue;
use jni::JavaVM;

/// Update notification ID 9001 used by the upstream background-service
/// plugin. The app-owned, nonexported Android provider builds a replacement
/// notification with the same ID/channel and an expanded lesson filename.
pub(crate) fn show_zip_lesson(lesson:&str,index:usize,total:usize) {
    if index==0 || total<index {return;}
    if let Err(error)=try_show_zip_lesson(lesson,index,total){
        eprintln!("Neural Nova: unable to refresh foreground lesson text: {error}");
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
    let name=env.new_string(lesson).map_err(|e|format!("Invalid lesson label: {e}"))?;
    let accepted=env.call_static_method(
        "com/qtech/neuralnova/experimental/NovaTrainingContextProvider",
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
