use bevy::prelude::bevy_main;
#[cfg(target_os = "android")]
use std::sync::OnceLock;

#[cfg(target_os = "android")]
static ANDROID_APP: OnceLock<android_activity::AndroidApp> = OnceLock::new();

#[cfg(target_os = "android")]
pub fn android_app() -> &'static android_activity::AndroidApp {
    ANDROID_APP.get().expect("AndroidApp not initialized before use")
}

#[bevy_main]
fn main() {
    // #[bevy_main] expands to android_main(app: AndroidApp) on Android
    // and plain main() on desktop. On Android the app arg is available
    // via a macro trick — we store it before calling run_app.
    #[cfg(target_os = "android")]
    {
        // bevy_main on Android generates:
        //   #[no_mangle] fn android_main(app: AndroidApp) { main_inner(app) }
        // We can't intercept it directly, so instead we use the ndk_context
        // which bevy/winit sets up for us. See android_permissions.rs.
    }
    valser::run_app();
}