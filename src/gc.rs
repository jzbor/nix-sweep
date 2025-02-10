use std::process;

pub fn gc() -> Result<(), String> {
    let result = process::Command::new("nix-store")
        .arg("--gc")
        .stdin(process::Stdio::inherit())
        .stdout(process::Stdio::inherit())
        .stderr(process::Stdio::inherit())
        .status();

    match result {
        Ok(status) => if status.success() {
            Ok(())
        } else {
            Err("Garbage collection failed".to_string())
        },
        Err(e) => Err(format!("Garbage collection failed ({})", e)),
    }
}
