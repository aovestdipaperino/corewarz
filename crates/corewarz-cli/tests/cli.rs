use std::process::Command;

fn corewarz() -> Command {
    Command::new(env!("CARGO_BIN_EXE_corewarz"))
}

#[test]
fn an_imp_beats_a_suicidal_warrior() {
    let out = corewarz()
        .args([
            "run",
            "warriors/imp.red",
            "warriors/suicide.red",
            "--positions",
            "0,4000",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR").to_string() + "/../..")
        .output()
        .expect("binary runs");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("imp"), "winner should be named: {text}");
    assert!(
        text.to_lowercase().contains("win"),
        "outcome should be reported: {text}"
    );
}

#[test]
fn assembly_errors_are_reported_with_a_line_number_and_a_failing_exit_code() {
    let dir = std::env::temp_dir().join("corewarz-cli-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("broken.red");
    std::fs::write(&path, "MOV.I $0, $1\nFROB $0, $0\n").unwrap();

    let out = corewarz()
        .args(["run", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!out.status.success(), "a broken warrior must fail");
    let text = String::from_utf8_lossy(&out.stderr);
    assert!(text.contains("line 2"), "must name the line: {text}");
    assert!(text.contains("FROB"), "must name the opcode: {text}");
}

#[test]
fn extended_opcodes_need_the_extended_flag() {
    let dir = std::env::temp_dir().join("corewarz-cli-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("ext.red");
    std::fs::write(&path, "AND.F $0, $1\nJMP $0, $0\n").unwrap();

    let classic = corewarz()
        .args(["run", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!classic.status.success());

    let extended = corewarz()
        .args([
            "run",
            path.to_str().unwrap(),
            "--extended",
            "--cycles",
            "100",
        ])
        .output()
        .unwrap();
    assert!(
        extended.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&extended.stderr)
    );
}

#[test]
fn the_same_seed_gives_the_same_result() {
    let run = |seed: &str| {
        corewarz()
            .args([
                "run",
                "warriors/imp.red",
                "warriors/dwarf.red",
                "--rounds",
                "5",
                "--seed",
                seed,
            ])
            .current_dir(env!("CARGO_MANIFEST_DIR").to_string() + "/../..")
            .output()
            .unwrap()
            .stdout
    };
    assert_eq!(run("7"), run("7"), "same seed must reproduce exactly");
}
