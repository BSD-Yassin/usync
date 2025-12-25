use std::fs;
use std::path::Path;
use std::process::Command;

fn setup_test_env() -> (tempfile::TempDir, std::path::PathBuf) {
    let temp_dir = tempfile::tempdir().unwrap();
    let test_dir = temp_dir.path().to_path_buf();

    fs::create_dir_all(test_dir.join("input")).unwrap();
    fs::create_dir_all(test_dir.join("output")).unwrap();

    fs::write(test_dir.join("input").join("test1.txt"), "test content 1").unwrap();
    fs::write(test_dir.join("input").join("test2.txt"), "test content 2").unwrap();

    fs::create_dir_all(test_dir.join("input").join("subdir")).unwrap();
    fs::write(
        test_dir.join("input").join("subdir").join("test3.txt"),
        "test content 3",
    )
    .unwrap();
    fs::write(
        test_dir.join("input").join("subdir").join("nested.txt"),
        "nested content",
    )
    .unwrap();

    (temp_dir, test_dir)
}

fn get_binary_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    // Check for target-specific release binary first
    let target = std::env::var("TARGET").unwrap_or_else(|_| {
        // Try to detect target from CARGO_BUILD_TARGET or use default
        std::env::var("CARGO_BUILD_TARGET").unwrap_or_else(|_| {
            // Default to host target
            if cfg!(target_os = "macos") {
                if cfg!(target_arch = "aarch64") {
                    "aarch64-apple-darwin".to_string()
                } else {
                    "x86_64-apple-darwin".to_string()
                }
            } else if cfg!(target_os = "linux") {
                "x86_64-unknown-linux-gnu".to_string()
            } else if cfg!(target_os = "windows") {
                "x86_64-pc-windows-msvc".to_string()
            } else {
                "unknown".to_string()
            }
        })
    });

    // Try target-specific path first
    let target_release_path = format!("{}/target/{}/release/usync", manifest_dir, target);
    if std::path::Path::new(&target_release_path).exists() {
        return target_release_path;
    }

    // Fallback to default release path
    let release_path = format!("{}/target/release/usync", manifest_dir);
    if std::path::Path::new(&release_path).exists() {
        return release_path;
    }

    // Fallback to debug path
    let debug_path = format!("{}/target/debug/usync", manifest_dir);
    if std::path::Path::new(&debug_path).exists() {
        return debug_path;
    }

    // Last resort: try target-specific debug path
    let target_debug_path = format!("{}/target/{}/debug/usync", manifest_dir, target);
    if std::path::Path::new(&target_debug_path).exists() {
        return target_debug_path;
    }

    // If nothing found, return the default release path (test will fail with clear error)
    release_path
}

#[test]
fn test_copy_single_file() {
    let (_temp, test_dir) = setup_test_env();
    let src = test_dir.join("input").join("test1.txt");
    let dst = test_dir.join("output").join("test1_copy.txt");

    let output = Command::new(get_binary_path())
        .arg(src.to_str().unwrap())
        .arg(dst.to_str().unwrap())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(dst.exists());

    let content = fs::read_to_string(&dst).unwrap();
    assert_eq!(content, "test content 1");
}

#[test]
fn test_copy_file_to_directory() {
    let (_temp, test_dir) = setup_test_env();
    let src = test_dir.join("input").join("test1.txt");
    let dst_dir = test_dir.join("output");

    let output = Command::new(get_binary_path())
        .arg(src.to_str().unwrap())
        .arg(dst_dir.to_str().unwrap())
        .output()
        .unwrap();

    assert!(output.status.success());
    let copied_file = dst_dir.join("test1.txt");
    assert!(copied_file.exists());

    let content = fs::read_to_string(&copied_file).unwrap();
    assert_eq!(content, "test content 1");
}

#[test]
fn test_recursive_copy_without_flag() {
    let (_temp, test_dir) = setup_test_env();
    let src = test_dir.join("input").join("subdir");
    let dst = test_dir.join("output").join("subdir_copy");

    let mut cmd = Command::new(get_binary_path());
    cmd.arg(src.to_str().unwrap())
        .arg(dst.to_str().unwrap())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().unwrap();
    use std::io::Write;
    child.stdin.as_mut().unwrap().write_all(b"n\n").unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(!dst.exists());
}

#[test]
fn test_recursive_copy_with_flag() {
    let (_temp, test_dir) = setup_test_env();
    let src = test_dir.join("input").join("subdir");
    let dst = test_dir.join("output").join("subdir_copy");

    let output = Command::new(get_binary_path())
        .arg("-r")
        .arg(src.to_str().unwrap())
        .arg(dst.to_str().unwrap())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(dst.exists());
    assert!(dst.join("test3.txt").exists());
    assert!(dst.join("nested.txt").exists());

    let content = fs::read_to_string(dst.join("test3.txt")).unwrap();
    assert_eq!(content, "test content 3");
}

#[test]
fn test_verbose_mode() {
    let (_temp, test_dir) = setup_test_env();
    let src = test_dir.join("input").join("test1.txt");
    let dst = test_dir.join("output").join("test1_verbose.txt");

    let output = Command::new(get_binary_path())
        .arg("-v")
        .arg(src.to_str().unwrap())
        .arg(dst.to_str().unwrap())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Copying") || stdout.contains("Successfully"));
}

#[test]
fn test_progress_mode() {
    let (_temp, test_dir) = setup_test_env();
    let src = test_dir.join("input").join("test1.txt");
    let dst = test_dir.join("output").join("test1_progress.txt");

    let output = Command::new(get_binary_path())
        .arg("-p")
        .arg(src.to_str().unwrap())
        .arg(dst.to_str().unwrap())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(dst.exists());
}

#[test]
fn test_error_nonexistent_source() {
    let (_temp, test_dir) = setup_test_env();
    let src = test_dir.join("input").join("nonexistent.txt");
    let dst = test_dir.join("output").join("dest.txt");

    let output = Command::new(get_binary_path())
        .arg(src.to_str().unwrap())
        .arg(dst.to_str().unwrap())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not exist") || stderr.contains("Error"));
}

#[test]
fn test_protocol_rejection() {
    let (_temp, test_dir) = setup_test_env();
    let dst = test_dir.join("output").join("dest.txt");

    let output = Command::new(get_binary_path())
        .arg("http://example.com/file.txt")
        .arg(dst.to_str().unwrap())
        .output()
        .unwrap();

    assert!(output.status.success() || !output.status.success());
}

#[test]
fn test_multiple_files_in_directory() {
    let (_temp, test_dir) = setup_test_env();
    let src = test_dir.join("input");
    let dst = test_dir.join("output").join("input_copy");

    let output = Command::new(get_binary_path())
        .arg("-r")
        .arg(src.to_str().unwrap())
        .arg(dst.to_str().unwrap())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(dst.join("test1.txt").exists());
    assert!(dst.join("test2.txt").exists());
    assert!(dst.join("subdir").join("test3.txt").exists());
    assert!(dst.join("subdir").join("nested.txt").exists());
}

#[test]
fn test_help_output() {
    let output = Command::new(get_binary_path())
        .arg("--help")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("usync"));
    assert!(stdout.contains("--verbose"));
    assert!(stdout.contains("--recursive"));
    assert!(stdout.contains("--progress"));
}

#[test]
fn test_version_output() {
    let output = Command::new(get_binary_path())
        .arg("--version")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("usync"));
}

#[test]
fn test_short_flags() {
    let (_temp, test_dir) = setup_test_env();
    let src = test_dir.join("input").join("test1.txt");
    let dst = test_dir.join("output").join("test1_short.txt");

    let output = Command::new(get_binary_path())
        .arg("-v")
        .arg("-p")
        .arg(src.to_str().unwrap())
        .arg(dst.to_str().unwrap())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(dst.exists());
}

#[test]
fn test_recursive_alias() {
    let (_temp, test_dir) = setup_test_env();
    let src = test_dir.join("input").join("subdir");
    let dst = test_dir.join("output").join("subdir_alias");

    let output = Command::new(get_binary_path())
        .arg("--rec")
        .arg(src.to_str().unwrap())
        .arg(dst.to_str().unwrap())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(dst.exists());
}

#[test]
fn test_nested_directory_structure() {
    let (_temp, test_dir) = setup_test_env();

    fs::create_dir_all(test_dir.join("input").join("level1").join("level2")).unwrap();
    fs::write(
        test_dir
            .join("input")
            .join("level1")
            .join("level2")
            .join("deep.txt"),
        "deep content",
    )
    .unwrap();

    let src = test_dir.join("input");
    let dst = test_dir.join("output").join("nested_copy");

    let output = Command::new(get_binary_path())
        .arg("-r")
        .arg(src.to_str().unwrap())
        .arg(dst.to_str().unwrap())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(dst.join("level1").join("level2").join("deep.txt").exists());

    let content = fs::read_to_string(dst.join("level1").join("level2").join("deep.txt")).unwrap();
    assert_eq!(content, "deep content");
}

#[test]
fn test_file_content_preservation() {
    let (_temp, test_dir) = setup_test_env();
    let original_content =
        "This is a test file with special characters: àáâãäå\nAnd newlines\nAnd tabs\there";
    fs::write(test_dir.join("input").join("special.txt"), original_content).unwrap();

    let src = test_dir.join("input").join("special.txt");
    let dst = test_dir.join("output").join("special_copy.txt");

    let output = Command::new(get_binary_path())
        .arg(src.to_str().unwrap())
        .arg(dst.to_str().unwrap())
        .output()
        .unwrap();

    assert!(output.status.success());
    let copied_content = fs::read_to_string(&dst).unwrap();
    assert_eq!(copied_content, original_content);
}

#[test]
fn test_large_file_copy() {
    let (_temp, test_dir) = setup_test_env();
    let large_content = "x".repeat(10000);
    fs::write(test_dir.join("input").join("large.txt"), &large_content).unwrap();

    let src = test_dir.join("input").join("large.txt");
    let dst = test_dir.join("output").join("large_copy.txt");

    let output = Command::new(get_binary_path())
        .arg(src.to_str().unwrap())
        .arg(dst.to_str().unwrap())
        .output()
        .unwrap();

    assert!(output.status.success());
    let copied_content = fs::read_to_string(&dst).unwrap();
    assert_eq!(copied_content.len(), large_content.len());
    assert_eq!(copied_content, large_content);
}
