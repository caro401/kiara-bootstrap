// Prevents additional console window on Windows in release, DO NOT REMOVE -- tauri!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::{ensure, Context};
use duct::cmd;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::api::process::{Command, CommandEvent};
use tauri::{AppHandle, Manager};

// hard code this python version for now
// if you change this, there's changes needed in pixi.toml
// and the definition file (get this from https://github.com/pyenv/pyenv/tree/master/plugins/python-build/share/python-build)
static PYTHON_VERSION: &str = "3.11.5";

fn start_main_app(env_vars: HashMap<String, String>) {
    let (mut rx, _child) = Command::new_sidecar("kiara-tauri")
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

fn log_to_frontend(handle: &AppHandle, message: impl Into<String>) {
    handle
        .emit_all(
            "logevent",
            Message {
                message: message.into(),
            },
        )
        .unwrap();
}

fn right_python_exists(kiara_appconfig_dir: &Path) -> bool {
    let python_command = kiara_appconfig_dir.to_owned().join("python/bin/python");
    let python_version_output = cmd!(python_command, "--version").stdout_capture().read();
    match python_version_output {
        Ok(output) => output.trim() == format!("Python {PYTHON_VERSION}"),
        Err(_) => false,
    }
}

fn get_resource_path(handle: &AppHandle, pathname: &str) -> PathBuf {
    handle
        .path_resolver()
        .resolve_resource(format!("resources/{pathname}"))
        .unwrap_or_else(|| panic!("failed to find {pathname} file in app resources"))
}
fn right_requirements_exist(handle: &AppHandle, kiara_appconfig_dir: &Path) -> bool {
    let existing_requirements_path = kiara_appconfig_dir.join("requirements.txt");
    let existing_requirements = std::fs::read_to_string(existing_requirements_path);
    let requirements_resource = get_resource_path(handle, "requirements.txt");
    let new_requierements = std::fs::read_to_string(requirements_resource).unwrap();
    existing_requirements.ok().as_ref() == Some(&new_requierements)
}

fn get_embedded_pixi() -> std::process::Command {
    // TODO get stdout from this guy
    Command::new_sidecar("pixi").unwrap().into()
}

fn compile_python(kiara_appconfig_dir: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(kiara_appconfig_dir)
        .context("Failed to create config directory for kiara apps")?;

    let install_result = get_embedded_pixi()
        .arg("install")
        .current_dir(kiara_appconfig_dir)
        .spawn()
        .context("Failed to pixi install. Do you have pixi on the system?")?
        .wait()
        .context("Something went wrong with pixi")?;
    ensure!(
        install_result.success(),
        "Failed to set up environment with pixi"
    );

    let python_result = get_embedded_pixi()
        .arg("run")
        .arg("compile-python")
        .current_dir(kiara_appconfig_dir)
        .spawn()
        .context("Failed to run pixi command. Do you have pixi on the system?")?
        .wait()
        .context("Something went wrong with pixi?")?;
    ensure!(python_result.success(), "Failed to compile python");
    Ok(())
}

fn pip_install(kiara_appconfig_dir: &Path) -> anyhow::Result<()> {
    let python_deps_result = std::process::Command::new("./python/bin/python")
        .args(["-m", "pip", "install", "-r", "requirements.txt"])
        .current_dir(kiara_appconfig_dir)
        .spawn()
        .context("Failed to run pip. Is your python install ok?")?
        .wait()
        .context("Something went wrong with installing packages")?;
    ensure!(python_deps_result.success(), "Failed to install kiara");
    Ok(())
}

fn copy_resource_file(handle: &AppHandle, kiara_appconfig_dir: &Path, filename: &str) {
    let resource = get_resource_path(handle, filename);
    std::fs::copy(resource, kiara_appconfig_dir.join(filename)).unwrap();
}

fn copy_resources(handle: &AppHandle, kiara_appconfig_dir: &Path) {
    std::fs::create_dir_all(kiara_appconfig_dir).unwrap();
    let files_to_copy = ["pixi.lock", "pixi.toml", PYTHON_VERSION, "requirements.txt"];
    for file in files_to_copy {
        copy_resource_file(handle, kiara_appconfig_dir, file);
    }
}

fn setup_python(handle: &AppHandle) -> anyhow::Result<HashMap<String, String>> {
    // define the config directory, we'll copy/install everything into $HOME/.kiara-app
    let mut kiara_appconfig_dir = dirs::home_dir().context("Failed to get home directory")?;
    kiara_appconfig_dir.push(".kiara-app");

    // is the existing python version the same?
    log_to_frontend(handle, "Checking for existing python...");
    let python_exists = right_python_exists(&kiara_appconfig_dir);
    if python_exists {
        log_to_frontend(handle, "Correct Python already exists");
    } else {
        log_to_frontend(
            handle,
            "Correct version of Python doesn't exist, installing... This might take a couple of minutes",
        );
        let _ = std::fs::remove_dir_all(&kiara_appconfig_dir);
        copy_resources(handle, &kiara_appconfig_dir);
        compile_python(&kiara_appconfig_dir)?;
        log_to_frontend(handle, "Python installed! Installing packages...");
        pip_install(&kiara_appconfig_dir)?;
    }

    log_to_frontend(handle, "Checking packages are up to date...");
    let packages_up_to_date = right_requirements_exist(handle, &kiara_appconfig_dir);
    if packages_up_to_date {
        log_to_frontend(handle, "Packages are up to date");
    } else {
        log_to_frontend(handle, "Updating packages...");
        copy_resources(handle, &kiara_appconfig_dir);
        pip_install(&kiara_appconfig_dir)?;
        log_to_frontend(handle, "Packages are up to date");
    }

    log_to_frontend(handle, "Starting network analysis app");
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

#[derive(Clone, serde::Serialize)]
struct Message {
    message: String,
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_secs(3));
                let result = setup_python(&handle);
                match result {
                    Ok(env_vars) => {
                        start_main_app(env_vars);
                        handle.get_window("main").unwrap().close().unwrap();
                    }
                    Err(error_text) => {
                        handle
                            .emit_all(
                                "errorevent",
                                Message {
                                    message: format!("{error_text:?}"),
                                },
                            )
                            .unwrap();
                    }
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
