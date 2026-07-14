use bevy::prelude::bevy_main;

#[bevy_main]
pub fn main() {
    // Get the Android internal files dir and pass it to the app
    #[cfg(target_os = "android")]
    {
        let ctx = ndk_context::android_context();
        let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.unwrap();
        let mut env = vm.attach_current_thread().unwrap();
        let activity = unsafe {
            jni::objects::JObject::from_raw(ctx.context().cast())
        };
        let files_dir = env
            .call_method(&activity, "getFilesDir", "()Ljava/io/File;", &[])
            .unwrap()
            .l()
            .unwrap();
        let path_str = env
            .call_method(&files_dir, "getAbsolutePath", "()Ljava/lang/String;", &[])
            .unwrap()
            .l()
            .unwrap();
        let path: String = env.get_string(&path_str.into()).unwrap().into();
        std::env::set_var("VALSER_DATA_DIR", &path);
    }

    valser::run_app();
}