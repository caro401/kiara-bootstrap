// Prevents additional console window on Windows in release, DO NOT REMOVE -- tauri!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use std::{fs, io};
use tauri::api::process::{Command, CommandEvent};

fn start_main_app(env_vars: HashMap<String, String>) {
    let (mut rx, mut child) = Command::new_sidecar("kiara-tauri")
        .expect("failed to create `kiara-tauri` binary command")
        .envs(env_vars)
        .spawn()
        .expect("Failed to spawn sidecar");

    tauri::async_runtime::spawn(async move {
        // get log messages from that app too
        while let Some(event) = rx.recv().await {
            if let CommandEvent::Stdout(line) = &event {
                println!("{line}")
            }
             if let CommandEvent::Stderr(line) = &event {
                eprintln!("{line}")
            }
        }
    });
}

fn hash_file_at_path(filepath: &Path) -> io::Result<String> {
    let mut hasher = Sha256::new();
    let mut file_to_hash = fs::File::open(filepath).unwrap();
    io::copy(&mut file_to_hash, &mut hasher).unwrap();
    Ok(base16::encode_lower(&hasher.finalize()))
}

fn setup_python(handle: tauri::AppHandle) -> io::Result<HashMap<String, String>> {
    // TODO SOME BETTER ERROR HANDLING HERE

    let pixi_lock_resource = handle
        .path_resolver()
        .resolve_resource("resources/pixi.lock")
        .expect("failed to find pixi.lock file");
    let pixi_toml_resource = handle
        .path_resolver()
        .resolve_resource("resources/pixi.toml")
        .expect("failed to find pixi.toml file");
    let python_definition_resource = handle
        .path_resolver()
        // TODO make this be less hardcoded
        .resolve_resource("resources/3.11.5")
        .expect("failed to find python definition");
    let python_deps_resource = handle
        .path_resolver()
        .resolve_resource("resources/requirements.txt")
        .expect("failed to find python package dependencies");
    let pixi_lock_hash = hash_file_at_path(&pixi_lock_resource).unwrap();
    let mut kiara_appconfig_dir = dirs::home_dir().expect("Failed to get home directory");
    kiara_appconfig_dir.push(".kiara-app");
    // TODO this isn't useful anymore now pixi is just the c compiler
    let existing_hash_path = kiara_appconfig_dir.join(".env_hash");
    let existing_hash = std::fs::read_to_string(&existing_hash_path);

    // either no python or not up to date
    // if existing_hash.ok().as_ref() != Some(&pixi_lock_hash) {
        // create the config directory if not exists
        std::fs::create_dir_all(&kiara_appconfig_dir).unwrap();
        // Copy in the pixi files
        std::fs::copy(pixi_lock_resource, kiara_appconfig_dir.join("pixi.lock")).unwrap();
        std::fs::copy(pixi_toml_resource, kiara_appconfig_dir.join("pixi.toml")).unwrap();
        std::fs::copy(python_definition_resource, kiara_appconfig_dir.join("3.11.5")).unwrap();
        std::fs::copy(python_deps_resource, kiara_appconfig_dir.join("requirements.txt")).unwrap();

        // delete .pixi directory from there if it exists
        // let _ because I explicitly don't care if it fails (better error handling later)
        let _ = std::fs::remove_dir_all(kiara_appconfig_dir.join(".pixi"));
        let _ = std::fs::remove_dir_all(kiara_appconfig_dir.join("python"));

        let install_result = std::process::Command::new("pixi")
            .arg("install")
            .current_dir(&kiara_appconfig_dir)
            .spawn()
            .expect("Failed to pixi install. Do you have pixi on the system.unwrap()")
            .wait()
            .expect("Something went wrong with pixi.unwrap()");
        assert!(
            install_result.success(),
            "Failed to set up environment with pixi"
        );
// TODO die if this fails
        std::fs::write(existing_hash_path, pixi_lock_hash).unwrap();

        let python_result = std::process::Command::new("pixi")
            .arg("run")
            .arg("compile-python")
            .current_dir(&kiara_appconfig_dir)
            .spawn()
            .expect("Failed to run pixi command. Do you have pixi on the system.unwrap()")
            .wait()
            .expect("Something went wrong with pixi.unwrap()");
        assert!(
            install_result.success(),
            "Failed to compile python"
        ); // TODO die if this fails
        let python_deps_result = std::process::Command::new("./python/bin/python")
            .args(["-m", "pip", "install", "-r", "requirements.txt" ])
            .current_dir(&kiara_appconfig_dir)
            .spawn()
            .expect("Failed to run pixi command. Do you have pixi on the system.unwrap()")
            .wait()
            .expect("Something went wrong with pixi.unwrap()");
        assert!(
            install_result.success(),
            "Failed to install kiara"
        ); // TODO die if this fails
    // }
    // else your env is already good, nothing to do

    let created_env_vars = [
        (
            "DYLD_LIBRARY_PATH".to_owned(),
            kiara_appconfig_dir
                .join("python/lib")
                .into_os_string()
                .into_string()
                .unwrap(),
        ),
        (
            "PYTHONPATH".to_owned(),
            kiara_appconfig_dir
                .join("python")
                .into_os_string()
                .into_string()
                .unwrap(),
        ),
    ];
    Ok(HashMap::from(created_env_vars))
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let env_vars = setup_python(app.handle()).unwrap();
            start_main_app(env_vars);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
