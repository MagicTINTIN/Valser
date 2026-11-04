use bevy::prelude::error;

#[cfg(target_os = "android")]
fn get_jni() -> Option<(jni::JavaVM, jni::objects::JObject<'static>)> {
    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.ok()?;
    let activity = unsafe { jni::objects::JObject::from_raw(ctx.context().cast()) };
    Some((vm, activity))
}

#[cfg(target_os = "android")]
pub fn request_audio_permission() {
    let Some((vm, activity)) = get_jni() else { return; };
    let mut env = vm.attach_current_thread().unwrap();

    // Check if we're on the main thread
    let looper_class = env.find_class("android/os/Looper").unwrap();
    let main_looper = env.call_static_method(&looper_class, "getMainLooper", "()Landroid/os/Looper;", &[]).unwrap().l().unwrap();
    let my_looper = env.call_static_method(&looper_class, "myLooper", "()Landroid/os/Looper;", &[]).unwrap().l().unwrap();
    let is_main = env.call_method(&main_looper, "equals", "(Ljava/lang/Object;)Z", &[(&my_looper).into()]).unwrap().z().unwrap();
    bevy::log::info!("request_audio_permission called on main thread: {is_main}");

    let Some((vm, activity)) = get_jni() else {
        bevy::log::error!("Could not get JNI context");
        return;
    };

    let mut env = match vm.attach_current_thread() {
        Ok(e) => e,
        Err(e) => {
            bevy::log::error!("JNI attach failed: {e:?}");
            return;
        }
    };

    let sdk_int = get_sdk_int(&mut env).unwrap_or(0);
    let permission = if sdk_int >= 33 {
        "android.permission.READ_MEDIA_AUDIO"
    } else {
        "android.permission.READ_EXTERNAL_STORAGE"
    };

    let perm_str = match env.new_string(permission) {
        Ok(s) => s,
        Err(e) => {
            bevy::log::error!("new_string failed: {e:?}");
            return;
        }
    };

    let already_granted = env
        .call_method(
            &activity,
            "checkSelfPermission",
            "(Ljava/lang/String;)I",
            &[(&perm_str).into()],
        )
        .and_then(|v| v.i())
        .unwrap_or(1);

    if already_granted == 0 {
        bevy::log::info!("Permission already granted");
        return;
    }

    bevy::log::info!("Requesting permission: {permission}");

    // Build the String[] array for the permission
    let string_class = match env.find_class("java/lang/String") {
        Ok(c) => c,
        Err(e) => {
            bevy::log::error!("find_class String failed: {e:?}");
            return;
        }
    };
    let perms_array = match env.new_object_array(1, &string_class, &perm_str) {
        Ok(a) => a,
        Err(e) => {
            bevy::log::error!("new_object_array failed: {e:?}");
            return;
        }
    };

    // --- THE FIX: wrap in a Runnable and call runOnUiThread ---
    //
    // requestPermissions() MUST be called on the UI thread.
    // From a native/Rust thread we must dispatch via runOnUiThread().

    // We need a GlobalRef so the Runnable lambda can capture the array
    let perms_global = match env.new_global_ref(&perms_array) {
        Ok(g) => g,
        Err(e) => {
            bevy::log::error!("new_global_ref failed: {e:?}");
            return;
        }
    };
    let activity_global = match env.new_global_ref(&activity) {
        Ok(g) => g,
        Err(e) => {
            bevy::log::error!("new_global_ref (activity) failed: {e:?}");
            return;
        }
    };

    // Use the Android Handler / post approach via reflection is complex;
    // the simplest correct approach is a Runnable anonymous class via JNI.
    // But since that requires generating a class at runtime, the easiest
    // workaround in pure JNI is to call runOnUiThread with a pre-built
    // Java lambda using a Handler posted to the main looper.

    // Approach: get main Looper, create a Handler, post a Runnable
    let looper_class = match env.find_class("android/os/Looper") {
        Ok(c) => c,
        Err(e) => {
            bevy::log::error!("find_class Looper failed: {e:?}");
            return;
        }
    };
    let main_looper = match env.call_static_method(
        &looper_class,
        "getMainLooper",
        "()Landroid/os/Looper;",
        &[],
    ) {
        Ok(v) => match v.l() {
            Ok(obj) => obj,
            Err(e) => {
                bevy::log::error!("getMainLooper .l() failed: {e:?}");
                return;
            }
        },
        Err(e) => {
            bevy::log::error!("getMainLooper failed: {e:?}");
            return;
        }
    };

    let handler_class = match env.find_class("android/os/Handler") {
        Ok(c) => c,
        Err(e) => {
            bevy::log::error!("find_class Handler failed: {e:?}");
            return;
        }
    };
    let handler = match env.new_object(
        &handler_class,
        "(Landroid/os/Looper;)V",
        &[(&main_looper).into()],
    ) {
        Ok(h) => h,
        Err(e) => {
            bevy::log::error!("new Handler failed: {e:?}");
            return;
        }
    };

    // The cleanest solution without bytecode generation:
    // Call activity.runOnUiThread() with a Runnable.
    //
    // Since we can't easily create anonymous Runnable in pure JNI without
    // a helper class, use your own tiny Java helper class (see below).
    // OR use the simpler approach: call requestPermissions directly but
    // ensure this whole function is invoked from the main thread.
    //
    // RECOMMENDED: add a tiny Java helper class to your Android project:

    let runnable_class = match env.find_class("fr/magictintin/valser/PermissionRunnable") {
        Ok(c) => c,
        Err(e) => {
            bevy::log::error!(
                "Could not find PermissionRunnable helper class: {e:?}. \
                 Make sure you added the Java helper class."
            );
            // Fallback: try calling directly (works if already on UI thread)
            let _ = env.call_method(
                &activity,
                "requestPermissions",
                "([Ljava/lang/String;I)V",
                &[(&perms_array).into(), 42i32.into()],
            );
            let _ = env.exception_clear();
            return;
        }
    };

    let runnable = match env.new_object(
        &runnable_class,
        "(Landroid/app/Activity;[Ljava/lang/String;I)V",
        &[
            (&activity_global).into(),
            (&perms_global).into(),
            42i32.into(),
        ],
    ) {
        Ok(r) => r,
        Err(e) => {
            bevy::log::error!("new PermissionRunnable failed: {e:?}");
            let _ = env.exception_clear();
            return;
        }
    };

    if let Err(e) = env.call_method(
        &activity,
        "runOnUiThread",
        "(Ljava/lang/Runnable;)V",
        &[(&runnable).into()],
    ) {
        bevy::log::error!("runOnUiThread failed: {e:?}");
        let _ = env.exception_clear();
    }
}

#[cfg(target_os = "android")]
fn get_sdk_int(env: &mut jni::JNIEnv) -> Option<i32> {
    let version_class = env.find_class("android/os/Build$VERSION").ok()?;
    env.get_static_field(&version_class, "SDK_INT", "I")
        .and_then(|v| v.i())
        .ok()
}



/// NOTE:
// #[cfg(target_os = "android")]
// pub fn request_audio_permission() {
//     use android_activity::AndroidApp;
//     // Bevy exposes the AndroidApp via a resource
//     // Request via the built-in API — no JNI needed
// }



// android_permissions.rs

// Embed the .dex at compile time — zero runtime file I/O needed
static PERMISSION_DEX: &[u8] = include_bytes!("../permission_runnable.dex");

#[cfg(target_os = "android")]
fn load_permission_runnable_class(env: &mut jni::JNIEnv) -> Option<jni::objects::JClass<'static>> {
    use std::io::Write;

    // Write .dex to app's cache dir so DexClassLoader can load it
    let cache_dir = {
        let ctx = ndk_context::android_context();
        let activity = unsafe { jni::objects::JObject::from_raw(ctx.context().cast()) };

        let cache_dir_file = env.call_method(
            &activity,
            "getCacheDir",
            "()Ljava/io/File;",
            &[],
        ).ok()?.l().ok()?;

        let path_obj = env.call_method(
            &cache_dir_file,
            "getAbsolutePath",
            "()Ljava/lang/String;",
            &[],
        ).ok()?.l().ok()?;

        let path: String = env.get_string(
            &jni::objects::JString::from(path_obj)
        ).ok()?.into();
        path
    };

    let dex_path = format!("{}/permission_runnable.dex", cache_dir);

    // Write dex bytes to cache dir
    if !std::path::Path::new(&dex_path).exists() {
        let mut f = std::fs::File::create(&dex_path).ok()?;
        f.write_all(PERMISSION_DEX).ok()?;
    }

    let dex_path_str = env.new_string(&dex_path).ok()?;
    let optimized_dir = env.new_string(&cache_dir).ok()?;
    let empty_str = env.new_string("").ok()?;

    // Use PathClassLoader (parent) — get it from the activity
    let ctx = ndk_context::android_context();
    let activity = unsafe { jni::objects::JObject::from_raw(ctx.context().cast()) };
    let parent_loader = env.call_method(
        &activity,
        "getClassLoader",
        "()Ljava/lang/ClassLoader;",
        &[],
    ).ok()?.l().ok()?;

    // Create a DexClassLoader to load our embedded dex
    let dcl_class = env.find_class("dalvik/system/DexClassLoader").ok()?;
    let loader = env.new_object(
        &dcl_class,
        "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/ClassLoader;)V",
        &[
            (&dex_path_str).into(),
            (&optimized_dir).into(),
            jni::objects::JValueGen::Object(&jni::objects::JObject::null()),
            (&parent_loader).into(),
        ],
    ).ok()?;

    // Load the class from our DexClassLoader
    let class_name = env.new_string("fr/magictintin/valser/PermissionRunnable").ok()?;
    let class_obj = env.call_method(
        &loader,
        "loadClass",
        "(Ljava/lang/String;)Ljava/lang/Class;",
        &[(&class_name).into()],  // note: loadClass uses dots not slashes
    ).ok()?.l().ok()?;

    // Cast to JClass
    Some(unsafe { jni::objects::JClass::from_raw(class_obj.into_raw()) })
}