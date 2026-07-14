use bevy::prelude::error;
 
#[cfg(target_os = "android")]
fn get_jni() -> Option<(jni::JavaVM, jni::objects::JObject<'static>)> {
    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.ok()?;
    // ctx.context() IS the Activity on NativeActivity (not just Application)
    // because NativeActivity IS an Activity subclass, not just a Context.
    let activity = unsafe { jni::objects::JObject::from_raw(ctx.context().cast()) };
    Some((vm, activity))
}

#[cfg(target_os = "android")]
pub fn request_audio_permission() {
    let Some((vm, activity)) = get_jni() else {
        bevy::log::error!("Could not get JNI context");
        return;
    };

    let mut env = match vm.attach_current_thread() {
        Ok(e) => e,
        Err(e) => { bevy::log::error!("JNI attach failed: {e:?}"); return; }
    };

    let sdk_int = get_sdk_int(&mut env).unwrap_or(0);
    let permission = if sdk_int >= 33 {
        "android.permission.READ_MEDIA_AUDIO"
    } else {
        "android.permission.READ_EXTERNAL_STORAGE"
    };

    let perm_str = match env.new_string(permission) {
        Ok(s) => s,
        Err(e) => { bevy::log::error!("new_string failed: {e:?}"); return; }
    };

    let already_granted = env.call_method(
        &activity,
        "checkSelfPermission",
        "(Ljava/lang/String;)I",
        &[(&perm_str).into()],
    ).and_then(|v| v.i()).unwrap_or(1);

    if already_granted != 0 {
        bevy::log::info!("Requesting permission: {permission}");
        let string_class = match env.find_class("java/lang/String") {
            Ok(c) => c,
            Err(_) => return,
        };
        let perms_array = match env.new_object_array(1, &string_class, &perm_str) {
            Ok(a) => a,
            Err(_) => return,
        };
        if let Err(e) = env.call_method(
            &activity,
            "requestPermissions",
            "([Ljava/lang/String;I)V",
            &[(&perms_array).into(), 1i32.into()],
        ) {
            bevy::log::error!("requestPermissions failed: {e:?}");
            // Clear any pending Java exception so we don't panic
            let _ = env.exception_clear();
        }
    } else {
        bevy::log::info!("Permission already granted");
    }
}

#[cfg(target_os = "android")]
pub fn check_audio_permission() -> bool {
    let Some((vm, activity)) = get_jni() else { return false; };
    let mut env = match vm.attach_current_thread() {
        Ok(e) => e,
        Err(_) => return false,
    };

    let sdk_int = get_sdk_int(&mut env).unwrap_or(0);
    let permission = if sdk_int >= 33 {
        "android.permission.READ_MEDIA_AUDIO"
    } else {
        "android.permission.READ_EXTERNAL_STORAGE"
    };

    let perm_str = match env.new_string(permission) {
        Ok(s) => s,
        Err(_) => return false,
    };

    env.call_method(
        &activity,
        "checkSelfPermission",
        "(Ljava/lang/String;)I",
        &[(&perm_str).into()],
    )
    .and_then(|v| v.i())
    .map(|r| r == 0)
    .unwrap_or(false)
}

#[cfg(target_os = "android")]
fn get_sdk_int(env: &mut jni::JNIEnv) -> Option<i32> {
    let version_class = env.find_class("android/os/Build$VERSION").ok()?;
    env.get_static_field(&version_class, "SDK_INT", "I")
        .and_then(|v| v.i())
        .ok()
}