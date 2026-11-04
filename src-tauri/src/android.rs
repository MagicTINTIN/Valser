use crossbeam_channel::Sender;
use jni::objects::{GlobalRef, JClass, JString, JValue};
use jni::sys::jlong;
use jni::{JNIEnv, JavaVM};
use std::sync::OnceLock;

use crate::engine::{Cmd, MediaState};

/// The Kotlin Bridge class, set when the media service starts.
static BRIDGE: OnceLock<(JavaVM, GlobalRef)> = OnceLock::new();
static COMMANDS: OnceLock<Sender<Cmd>> = OnceLock::new();

pub fn listen(tx: Sender<Cmd>) {
    let _ = COMMANDS.set(tx);
}

#[unsafe(no_mangle)]
extern "system" fn Java_fr_magictintin_valser_Bridge_attach(env: JNIEnv, class: JClass) {
    if let (Ok(vm), Ok(class)) = (env.get_java_vm(), env.new_global_ref(class)) {
        let _ = BRIDGE.set((vm, class));
    }
}

#[unsafe(no_mangle)]
extern "system" fn Java_fr_magictintin_valser_Bridge_command(
    mut env: JNIEnv,
    _: JClass,
    name: JString,
    value: jlong,
) {
    let Ok(name) = env.get_string(&name).map(String::from) else { return };
    let cmd = match name.as_str() {
        "play" => Cmd::Resume,
        "pause" => Cmd::Pause,
        "toggle" => Cmd::Toggle,
        "next" => Cmd::Next,
        "prev" => Cmd::Prev,
        "stop" => Cmd::Stop,
        "seek" => Cmd::Seek { secs: value as f64 / 1000.0 },
        _ => return,
    };
    if let Some(tx) = COMMANDS.get() {
        let _ = tx.send(cmd);
    }
}

pub fn update(media: &MediaState) {
    let Some((vm, class)) = BRIDGE.get() else { return };
    let Ok(mut env) = vm.attach_current_thread_permanently() else { return };
    let result = env.with_local_frame(4, |env| -> jni::errors::Result<()> {
        let title = env.new_string(&media.title)?;
        let artist = env.new_string(&media.artist)?;
        let class: &JClass = class.as_obj().into();
        env.call_static_method(
            class,
            "update",
            "(ILjava/lang/String;Ljava/lang/String;JJ)V",
            &[
                JValue::Int(media.state as i32),
                JValue::Object(&title),
                JValue::Object(&artist),
                JValue::Long((media.length * 1000.0) as i64),
                JValue::Long((media.position * 1000.0) as i64),
            ],
        )?;
        Ok(())
    });
    if result.is_err() {
        let _ = env.exception_clear();
    }
}
