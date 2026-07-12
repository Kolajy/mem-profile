use std::env;
use std::process::{Command, exit};

pub fn execute(args: Vec<String>) {
    let current_exe = env::current_exe().expect("Failed to get current executable path");
    let exe_path = current_exe
        .display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");

    let runner_config = format!("target.'cfg(all())'.runner=[\"{}\", \"run\"]", exe_path);

    let mut cmd = Command::new("cargo");
    cmd.arg("--config");
    cmd.arg(&runner_config);
    cmd.args(&args);

    let status = cmd.status().unwrap_or_else(|err| {
        eprintln!("Failed to execute cargo command: {}", err);
        exit(1);
    });

    if !status.success() {
        if let Some(code) = status.code() {
            exit(code);
        } else {
            exit(1);
        }
    }
}
