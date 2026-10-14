//! Android-only checkpoint export to a user-selected SAF content:// URI.
//! We deliberately let Android Java hold the ContentResolver OutputStream
//! open until close(), then reopen and SHA-256 verify the saved document.
//! This avoids the Android zero-byte metadata bug in certain Tauri fs versions.
use jni::objects::{JClass,JObject,JString,JValue};
use jni::JavaVM;
use std::path::Path;

pub(crate) fn write_android_document(source:&Path,uri:&str)->Result<u64,String>{
    if !uri.starts_with("content://"){
        return Err("Select an Android Files document destination".into());
    }
    let source=source.to_str().ok_or("Checkpoint path cannot be represented in UTF-8")?;
    let context=ndk_context::android_context();
    // SAFETY: ndk-context stores Tauri's process-wide JavaVM pointer.
    let vm=unsafe{JavaVM::from_raw(context.vm() as *mut jni::sys::JavaVM)}
        .map_err(|e|format!("Android JavaVM unavailable: {e}"))?;
    let mut env=vm.attach_current_thread()
        .map_err(|e|format!("Unable to attach checkpoint export thread: {e}"))?;
    // The application Context is owned by the process, not this JNI scope.
    let app_context=std::mem::ManuallyDrop::new(unsafe{
        JObject::from_raw(context.context() as jni::sys::jobject)
    });
    let loader=env.call_method(&*app_context,"getClassLoader",
        "()Ljava/lang/ClassLoader;",&[])
        .map_err(|e|format!("Android application class loader missing: {e}"))?
        .l().map_err(|e|e.to_string())?;
    let class_name=env.new_string("com.qtech.neuralnova.experimental.NovaTrainingContextProvider")
        .map_err(|e|e.to_string())?;
    let provider=env.call_method(loader,"loadClass",
        "(Ljava/lang/String;)Ljava/lang/Class;",
        &[JValue::Object(class_name.as_ref())])
        .map_err(|e|format!("Android checkpoint writer class unavailable: {e}"))?
        .l().map_err(|e|e.to_string())?;
    let provider=JClass::from(provider);
    let source_path=env.new_string(source)
        .map_err(|e|format!("Could not encode checkpoint source: {e}"))?;
    let document_uri=env.new_string(uri)
        .map_err(|e|format!("Could not encode checkpoint destination: {e}"))?;
    let outcome=env.call_static_method(provider,"writeVerifiedCheckpoint",
        "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
        &[JValue::Object(source_path.as_ref()),JValue::Object(document_uri.as_ref())])
        .map_err(|e|format!("Android native checkpoint write failed: {e}"))?
        .l().map_err(|e|e.to_string())?;
    let outcome=JString::from(outcome);
    let response:String=env.get_string(&outcome)
        .map_err(|e|format!("Invalid Android checkpoint export result: {e}"))?
        .into();
    if let Some(count)=response.strip_prefix("OK:"){
        return count.parse::<u64>().map_err(|e|format!("Invalid saved size: {e}"));
    }
    Err(response.strip_prefix("ERROR:").unwrap_or(&response).to_owned())
}
