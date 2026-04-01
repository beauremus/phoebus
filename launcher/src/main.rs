use std::env;
use std::process::Command;
use urlencoding::decode;

fn main() {
    let args: Vec<String> = env::args().collect();

    // If run with no arguments (user double-click), register the protocol
    if args.len() < 2 {
        match register_protocol() {
            Ok(_) => {
                println!("========================================");
                println!("   PHOEBUS URI HANDLER REGISTERED       ");
                println!("========================================");
                println!("You can now launch .bob files from the");
                println!("Flutter demo app using phoebus:// paths.");
                println!("\nPress Enter to exit...");
                let _ = std::io::stdin().read_line(&mut String::new());
            }
            Err(e) => eprintln!("Failed to register URI: {}", e),
        }
        return;
    }

    let raw_uri = &args[1];
    let clean_uri = raw_uri
        .trim_start_matches("phoebus://")
        .trim_start_matches("phoebus:");
    let decoded_path = decode(clean_uri).unwrap_or(clean_uri.into()).into_owned();

    launch_phoebus(&decoded_path);
}

#[cfg(target_os = "windows")]
fn register_protocol() -> Result<(), Box<dyn std::error::Error>> {
    use winreg::RegKey;
    use winreg::enums::*;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey("Software\\Classes\\phoebus")?;
    key.set_value("", &"URL:Phoebus Protocol")?;
    key.set_value("URL Protocol", &"")?;

    let (cmd_key, _) = key.create_subkey("shell\\open\\command")?;
    let current_exe = env::current_exe()?;
    let current_exe_str = current_exe.to_str().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Executable path is not valid UTF-8",
        )
    })?;
    cmd_key.set_value("", &format!("\"{}\" \"%1\"", current_exe_str))?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn register_protocol() -> Result<(), Box<dyn std::error::Error>> {
    let home = env::var("HOME")?;
    let desktop_dir = format!("{}/.local/share/applications", home);
    let current_exe = env::current_exe()?;
    let current_exe_str = current_exe.to_str().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Executable path is not valid UTF-8",
        )
    })?;

    // Per the Desktop Entry Specification, the Exec value must be quoted and
    // special characters escaped when the path contains spaces or shell metacharacters.
    let quoted_exe = desktop_entry_quote(current_exe_str);

    let content = format!(
        "[Desktop Entry]\nType=Application\nName=Phoebus Launcher\nExec={} %u\nMimeType=x-scheme-handler/phoebus;\nNoDisplay=true",
        quoted_exe
    );

    let desktop_file_name = "phoebus.desktop";
    let desktop_file_path = format!("{}/{}", desktop_dir, desktop_file_name);

    std::fs::create_dir_all(&desktop_dir)?;
    std::fs::write(&desktop_file_path, content)?;
    Command::new("update-desktop-database")
        .arg(&desktop_dir)
        .status()?;
    Command::new("xdg-mime")
        .arg("default")
        .arg(desktop_file_name)
        .arg("x-scheme-handler/phoebus")
        .status()?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn register_protocol() -> Result<(), Box<dyn std::error::Error>> {
    // macOS registration is handled by the Info.plist in the App Bundle.
    // We just return Ok so the launcher can still be run to verify paths.
    println!("macOS detected: Registration is handled via the App Bundle structure.");
    Ok(())
}

/// Quote a path for use in a Desktop Entry `Exec` field per the Desktop Entry Specification.
/// Wraps the value in double-quotes and escapes the characters that must be escaped inside
/// double-quoted strings: `"`, `` ` ``, `$`, and `\`.
#[cfg(target_os = "linux")]
fn desktop_entry_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' | '`' | '$' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn register_protocol() -> Result<(), Box<dyn std::error::Error>> {
    Err(
        "Protocol registration is not supported on this operating system. \
         Please run this launcher on Windows, Linux, or macOS."
            .into(),
    )
}
fn launch_phoebus(resource_path: &str) {
    let exe_path = env::current_exe().expect("Failed to get current exe path");
    let bin_dir = exe_path.parent().expect("Failed to get exe directory");

    #[cfg(target_os = "windows")]
    {
        let script_path = bin_dir.join("phoebus.bat");
        Command::new("cmd")
            .arg("/c")
            .arg(&script_path)
            .arg("-server")
            .arg("-resource")
            .arg(resource_path)
            .spawn()
            .expect("Failed to launch Phoebus batch script");
    }

    #[cfg(not(target_os = "windows"))]
    {
        let script_path = bin_dir.join("phoebus.sh");
        // We call the script directly instead of 'sh -c' to ensure args forward correctly
        Command::new(script_path)
            .arg("-server")
            .arg("-resource")
            .arg(resource_path)
            .spawn()
            .expect("Failed to launch Phoebus shell script");
    }
}
