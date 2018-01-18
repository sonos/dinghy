use std::env;
use std::ffi::OsStr;
use std::ffi::OsString;
use super::Result;

pub fn append_path_to_env<K: AsRef<OsStr>, V: AsRef<OsStr>>(key: K, value: V) {
    debug!("Appending {:?} to {:?}", value.as_ref(), key.as_ref());
    let mut formatted_value = OsString::new();
    if let Ok(initial_value) = env::var(key.as_ref()) {
        formatted_value.push(initial_value);
        formatted_value.push(":");
    }
    formatted_value.push(value);
    env::set_var(key.as_ref(), formatted_value);
}

pub fn append_path_to_target_env<K: AsRef<OsStr>, R: AsRef<str>, V: AsRef<OsStr>>(
    k: K, rustc_triple: Option<R>, v: V) {
    append_path_to_env(target_key_from_triple(k, rustc_triple), v.as_ref())
}

pub fn env_rerun_if_changed(name: &str) -> Result<String> {
    println!("cargo:rerun-if-env-changed={}", name);
    Ok(env::var(name)?)
}

pub fn envify<S: AsRef<str>>(name: S) -> String {
    name.as_ref()
        .chars()
        .map(|c| c.to_ascii_uppercase())
        .map(|c| { if c == '-' || c == '.' { '_' } else { c } })
        .collect()
}

pub fn set_all_env<K: AsRef<OsStr>, V: AsRef<OsStr>>(env: &[(K, V)]) {
    for env_var in env {
        set_env(env_var.0.as_ref(), env_var.1.as_ref())
    }
}

pub fn set_env<K: AsRef<OsStr>, V: AsRef<OsStr>>(k: K, v: V) {
    debug!("Setting environment variable {:?}={:?}", k.as_ref(), v.as_ref());
    env::set_var(k, v);
}

pub fn set_env_ifndef<K: AsRef<OsStr>, V: AsRef<OsStr>>(k: K, v: V) {
    if let Ok(current_env_value) = env::var(k.as_ref()) {
        debug!("Ignoring value {:?} as environment variable {:?} already defined with value {:?}",
               k.as_ref(), v.as_ref(), current_env_value);
    } else {
        debug!("Setting environment variable {:?}={:?}", k.as_ref(), v.as_ref());
        env::set_var(k, v);
    }
}

pub fn set_target_env<K: AsRef<OsStr>, R: AsRef<str>, V: AsRef<OsStr>>(k: K, rustc_triple: Option<R>, v: V) {
    set_env(target_key_from_triple(k, rustc_triple), v);
}

pub fn target_env(var_base: &str) -> Result<String> {
    if let Ok(target) = env::var("TARGET") {
        let is_host = env::var("HOST")? == target;
        target_env_from_triple(var_base, target.as_str(), is_host)
    } else {
        env_rerun_if_changed(var_base)
    }
}

fn target_env_from_triple(var_base: &str, triple: &str, is_host: bool) -> Result<String> {
    env_rerun_if_changed(&format!("{}_{}", var_base, triple))
        .or_else(|_| env_rerun_if_changed(&format!("{}_{}", var_base, triple.replace("-", "_"))))
        .or_else(|_| env_rerun_if_changed(&format!("{}_{}", if is_host { "HOST" } else { "TARGET" }, var_base)))
        .or_else(|_| env_rerun_if_changed(var_base))
}

fn target_key_from_triple<K: AsRef<OsStr>, R: AsRef<str>>(k: K, rustc_triple: Option<R>) -> OsString {
    let mut target_key = OsString::new();
    target_key.push(k);
    if let Some(rustc_triple) = rustc_triple {
        target_key.push("_");
        target_key.push(rustc_triple.as_ref().replace("-", "_"));
    }
    target_key
}
