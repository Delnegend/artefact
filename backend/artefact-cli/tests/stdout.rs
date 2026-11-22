use std::{path::PathBuf, process::Command};

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../artefact-core/tests/fixtures/baseline_420.jpg")
}

#[test]
fn stdout_contains_only_the_requested_image() {
    for (format, sixteen_bit) in [("png", false), ("png", true), ("tiff", false), ("jxl", true)] {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_artefact-cli"));
        cmd.arg(fixture()).args(["-o", "-", "--format", format, "-i", "1"]);
        if sixteen_bit {
            cmd.arg("--sixteen-bit");
        }
        let output = cmd.output().expect("run CLI");
        assert!(output.status.success(), "{format}: {}", String::from_utf8_lossy(&output.stderr));
        match format {
            "png" => {
                assert!(output.stdout.starts_with(b"\x89PNG\r\n\x1a\n"));
                assert_eq!(output.stdout[24], if sixteen_bit { 16 } else { 8 });
            }
            "tiff" => assert!(output.stdout.starts_with(b"II*\0") || output.stdout.starts_with(b"MM\0*")),
            "jxl" => assert!(output.stdout.starts_with(&[0xff, 0x0a])),
            _ => panic!("unexpected format {format}"),
        }
        assert!(String::from_utf8_lossy(&output.stderr).contains("solving on CPU"));
    }
}

#[test]
fn stdout_rejects_invalid_format_without_writing_image() {
    let output = Command::new(env!("CARGO_BIN_EXE_artefact-cli"))
        .arg(fixture())
        .args(["-o", "-", "--format", "webp", "--sixteen-bit"])
        .output()
        .expect("run CLI");
    assert!(!output.status.success());
    assert_eq!(output.stdout, []);
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires PNG or JPEG XL"));
}
