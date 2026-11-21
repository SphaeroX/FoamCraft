// SPDX-License-Identifier: GPL-3.0-or-later
use foamcraft_agent::tail_stage;

fn dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("fc-tail-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn returns_only_complete_lines_while_running() {
    let d = dir("run");
    std::fs::write(d.join("log"), "Time = 1\nTime = 2\nTime").unwrap();
    let r = tail_stage(&d.join("log"), &d.join("exit"), 0);
    assert_eq!(r.text, "Time = 1\nTime = 2\n");
    assert_eq!(r.next_offset, 18);
    assert_eq!(r.exit, None);
    let r = tail_stage(&d.join("log"), &d.join("exit"), 9);
    assert_eq!(r.text, "Time = 2\n");
}

#[test]
fn returns_everything_and_the_exit_code_when_finished() {
    let d = dir("done");
    std::fs::write(d.join("log"), "a\nb").unwrap();
    std::fs::write(d.join("exit"), "3\n").unwrap();
    let r = tail_stage(&d.join("log"), &d.join("exit"), 2);
    assert_eq!(r.text, "b");
    assert_eq!(r.exit, Some(3));
}

#[test]
fn missing_log_is_empty() {
    let d = dir("none");
    let r = tail_stage(&d.join("log"), &d.join("exit"), 0);
    assert_eq!((r.text.as_str(), r.next_offset, r.exit), ("", 0, None));
}
