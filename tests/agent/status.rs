// SPDX-License-Identifier: GPL-3.0-or-later
use foamcraft_agent::{effective_memory, parse_api_info, parse_cgroup_memory_max, parse_meminfo};

#[test]
fn reads_total_memory_from_meminfo() {
    let text = "MemTotal:        2003504 kB\nMemFree:          123456 kB\n";
    assert_eq!(parse_meminfo(text), Some(2_003_504 * 1024));
    assert_eq!(parse_meminfo("garbage"), None);
}

#[test]
fn reads_cgroup_memory_limit() {
    assert_eq!(parse_cgroup_memory_max("1073741824\n"), Some(1_073_741_824));
    assert_eq!(parse_cgroup_memory_max("max\n"), None);
}

#[test]
fn memory_limit_wins_when_smaller() {
    assert_eq!(effective_memory(Some(8), Some(4)), Some(4));
    assert_eq!(effective_memory(Some(8), None), Some(8));
    assert_eq!(effective_memory(Some(8), Some(16)), Some(8));
    assert_eq!(effective_memory(None, Some(4)), Some(4));
}

#[test]
fn reads_openfoam_api_info() {
    let info = parse_api_info("api=2606\npatch=0\n", "build=_481094f-20260618\n");
    assert_eq!(info.api, "2606");
    assert_eq!(info.patch, "0");
    assert_eq!(info.build, "_481094f-20260618");
    assert_eq!(info.version(), "v2606");
}

#[test]
fn finds_openfoam_without_environment() {
    let root = std::env::temp_dir().join(format!("fc-agent-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("openfoam2606/META-INFO")).unwrap();
    std::fs::create_dir_all(root.join("unrelated")).unwrap();
    let found = foamcraft_agent::openfoam_project_dir(None, &root).unwrap();
    assert!(found.ends_with("openfoam2606"));
    let from_env = foamcraft_agent::openfoam_project_dir(Some("/opt/of"), &root).unwrap();
    assert_eq!(from_env, std::path::PathBuf::from("/opt/of"));
    let empty = root.join("unrelated");
    assert!(foamcraft_agent::openfoam_project_dir(None, &empty).is_none());
    std::fs::remove_dir_all(&root).unwrap();
}
