use super::*;

fn proc(name: &str, path: &str, cmd: &[&str]) -> Proc {
    Proc {
        pid: 100,
        parent: None,
        name: name.to_string(),
        exe: Some(PathBuf::from(path)),
        exe_lower: path.to_lowercase(),
        cmd: cmd.iter().map(|s| s.to_string()).collect(),
        mem: 10 * MB,
    }
}

fn def(id: &str) -> &'static TargetDef {
    TARGETS.iter().find(|t| t.id == id).expect("target exists")
}

fn custom(id: &str, processes: &[&str], services: &[&str]) -> CustomTarget {
    CustomTarget {
        id: id.to_string(),
        label: id.to_string(),
        processes: processes.iter().map(|s| s.to_string()).collect(),
        services: services.iter().map(|s| s.to_string()).collect(),
        ..CustomTarget::default()
    }
}

fn mode(id: &str, targets: &[&str]) -> ModeConfig {
    ModeConfig {
        id: id.to_string(),
        name: id.to_uppercase(),
        targets: targets.iter().map(|s| s.to_string()).collect(),
        ..ModeConfig::default()
    }
}

#[test]
fn target_ids_are_unique_and_services_are_plain_names() {
    let mut seen = HashSet::new();
    for t in TARGETS {
        assert!(seen.insert(t.id), "duplicate target id {}", t.id);
        assert!(!t.label.is_empty());
        for name in t.services {
            assert!(!name.contains('\'') && !name.contains('"'), "unsafe service name {name}");
        }
    }
}

#[test]
fn presets_only_name_public_launchers() {
    // Game Mode must not ship someone's personal setup. The presets are launchers that need
    // special handling and nothing else.
    let ids: Vec<&str> = TARGETS.iter().map(|t| t.id).collect();
    assert_eq!(ids, vec!["discord", "spotify", "steam", "epic", "riotclient", "docker_wsl"]);
    assert!(TARGETS.iter().all(|t| t.services.is_empty()), "presets must not stop services");
}

#[test]
fn default_config_closes_nothing_until_the_user_picks() {
    let cfg = GameModeConfig::default();
    assert_eq!(cfg.modes.len(), 1);
    assert_eq!(cfg.active_mode, cfg.modes[0].id);
    assert!(cfg.modes[0].targets.is_empty(), "a new mode must start with nothing selected");
    assert!(cfg.modes[0].power_plan.is_empty(), "a new mode must leave the power plan alone");
    assert_eq!(cfg.modes[0].tweaks, vec!["priority".to_string()]);
    assert!(cfg.custom_targets.is_empty());
    assert!(cfg.auto_detect);
    assert!(!cfg.arm_on_launch);
    assert_eq!(cfg.restore, "all");
    assert_eq!(cfg.elevate, "ask");
    assert!(cfg.settle_seconds >= 10, "an automatic start must wait for the game to settle");
}

#[test]
fn sanitize_drops_unknown_ids_and_bad_values() {
    let cfg = sanitize(GameModeConfig {
        modes: vec![ModeConfig {
            id: "Comp!".into(),
            name: "  Competitive  ".into(),
            targets: vec!["discord".into(), "discord".into(), "format_disk".into()],
            tweaks: vec!["priority".into(), "nope".into()],
            power_plan: "{29020666-3A1B-4B66-9D4C-B33D99302449}".into(),
            start_apps: vec![
                StartApp { label: "".into(), path: r#" "C:\Program Files\ExitLag\ExitLag.exe" "#.into(), args: "".into(), close_on_end: true },
                StartApp { label: "Dup".into(), path: r"c:\program files\exitlag\exitlag.exe".into(), args: "".into(), close_on_end: false },
                StartApp { label: "Not a program".into(), path: r"C:\Users\me\notes.txt".into(), args: "".into(), close_on_end: false },
                StartApp { label: "Explorer".into(), path: r"C:\Windows\explorer.exe".into(), args: "".into(), close_on_end: true },
            ],
        }],
        active_mode: "missing".into(),
        settle_seconds: 9999,
        restore: "weird".into(),
        elevate: "root".into(),
        ..GameModeConfig::default()
    });
    assert_eq!(cfg.modes[0].id, "comp");
    assert_eq!(cfg.modes[0].name, "Competitive");
    assert_eq!(cfg.modes[0].targets, vec!["discord".to_string()]);
    assert_eq!(cfg.modes[0].tweaks, vec!["priority".to_string()]);
    assert_eq!(cfg.modes[0].power_plan, "29020666-3a1b-4b66-9d4c-b33d99302449");
    // Quotes are trimmed, duplicates, non-programs and protected programs are dropped.
    assert_eq!(cfg.modes[0].start_apps.len(), 1);
    assert_eq!(cfg.modes[0].start_apps[0].path, r"C:\Program Files\ExitLag\ExitLag.exe");
    assert_eq!(cfg.modes[0].start_apps[0].label, "exitlag");
    assert_eq!(cfg.active_mode, "comp");
    assert_eq!(cfg.settle_seconds, 120);
    assert_eq!(cfg.restore, "all");
    assert_eq!(cfg.elevate, "ask");

    let bad_plan = sanitize(GameModeConfig {
        modes: vec![ModeConfig { power_plan: "balanced; calc.exe".into(), ..mode("a", &[]) }],
        ..GameModeConfig::default()
    });
    assert!(bad_plan.modes[0].power_plan.is_empty(), "only a real GUID may reach powercfg");
}

#[test]
fn old_config_files_become_the_first_mode() {
    let raw = r#"{"targets":["discord","bogus"],"tweaks":["priority"],"autoDetect":false,"restore":"all","elevate":"uac"}"#;
    let cfg = sanitize(parse_config(raw).expect("old file parses"));
    assert_eq!(cfg.modes.len(), 1);
    assert_eq!(cfg.modes[0].targets, vec!["discord".to_string()]);
    assert_eq!(cfg.modes[0].tweaks, vec!["priority".to_string()]);
    assert!(cfg.modes[0].power_plan.is_empty());
    assert!(!cfg.auto_detect);
    assert_eq!(cfg.elevate, "ask");
    assert!(cfg.targets.is_none() && cfg.tweaks.is_none());
    // A file with modes keeps them, and a mode saved before power plans existed leaves the plan alone.
    let kept = sanitize(parse_config(r#"{"modes":[{"id":"a","name":"A","targets":["steam"],"tweaks":[]}]}"#).unwrap());
    assert_eq!(kept.modes[0].targets, vec!["steam".to_string()]);
    assert!(kept.modes[0].power_plan.is_empty());
}

#[test]
fn custom_targets_cannot_reach_protected_programs_or_services() {
    let cfg = sanitize(GameModeConfig {
        custom_targets: vec![
            custom("mine", &["Obsidian.exe", "explorer", "vgc", "bad;name"], &["MyService", "Audiosrv", "vgk", "nvdisplay.containerlocalsystem", "x'y"]),
            custom("empty", &["explorer"], &["WinDefend"]),
            custom("discord", &["discord"], &[]),
            custom("dup", &["a"], &[]),
            custom("dup", &["b"], &[]),
        ],
        ..GameModeConfig::default()
    });
    let ids: Vec<&str> = cfg.custom_targets.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, vec!["mine", "dup"], "built-in ids, duplicates and targets with nothing left are dropped");
    assert_eq!(cfg.custom_targets[0].processes, vec!["obsidian".to_string()]);
    assert_eq!(cfg.custom_targets[0].services, vec!["MyService".to_string()]);
    assert!(service_denied("EasyAntiCheat_EOS") && service_denied("NVDisplay.ContainerLocalSystem") && service_denied("WinDefend"));
    assert!(!service_denied("MyService"));
}

#[test]
fn modes_only_reference_known_targets_and_a_custom_target_matches_by_name() {
    let cfg = sanitize(GameModeConfig {
        custom_targets: vec![custom("notes", &["obsidian"], &[])],
        modes: vec![
            mode("a", &["notes", "ghost"]),
            ModeConfig { name: "dup".into(), ..mode("a", &[]) },
            ModeConfig { tweaks: vec!["priority".into()], ..mode("b", &[]) },
        ],
        active_mode: "b".into(),
        ..GameModeConfig::default()
    });
    assert_eq!(cfg.modes.len(), 2);
    assert_eq!(cfg.modes[0].targets, vec!["notes".to_string()]);
    assert_eq!(pick_mode(&cfg, Some("a")).name, "A");
    assert_eq!(pick_mode(&cfg, Some("nope")).name, "B");
    assert_eq!(pick_mode(&cfg, None).name, "B");

    let defs = all_targets(&cfg);
    assert_eq!(defs.len(), TARGETS.len() + 1);
    let notes = defs.iter().find(|d| d.id == "notes").expect("custom target is listed");
    assert!(matches(notes, &proc("obsidian", "C:\\Apps\\Obsidian\\Obsidian.exe", &[])));
    assert!(!matches(notes, &proc("figma", "C:\\Apps\\figma.exe", &[])));
    // The same config gives the same leaked definition instead of leaking again.
    assert!(std::ptr::eq(*notes, *all_targets(&cfg).iter().find(|d| d.id == "notes").unwrap()));
}

#[test]
fn a_custom_target_for_a_dev_runtime_spares_deckforges_own_tools() {
    let cfg = sanitize(GameModeConfig {
        custom_targets: vec![custom("node", &["node"], &[])],
        ..GameModeConfig::default()
    });
    let defs = all_targets(&cfg);
    let node = defs.iter().find(|d| d.id == "node").expect("custom target is listed");
    let vite = proc("node", "C:\\Program Files\\nodejs\\node.exe", &["node", "vite.js"]);
    let tauri = proc("node", "C:\\Program Files\\nodejs\\node.exe", &["node", "tauri", "dev"]);
    let other = proc("node", "C:\\Program Files\\nodejs\\node.exe", &["node", "server.js"]);
    assert!(is_dev_stack(&vite) && is_dev_stack(&tauri) && !is_dev_stack(&other));
    let procs = vec![vite, tauri, other];
    assert_eq!(matched_procs(node, &procs, &HashSet::new()).len(), 1);
}

#[test]
fn anti_cheat_and_system_processes_are_never_matched() {
    assert!(never_kill(&proc("vgc", "C:\\Program Files\\Riot Vanguard\\vgc.exe", &[])));
    assert!(never_kill(&proc("explorer", "C:\\Windows\\explorer.exe", &[])));
    assert!(never_kill(&proc("anything", "C:\\Windows\\System32\\anything.exe", &[])));
    assert!(never_kill(&proc("easyanticheat_eos", "C:\\Games\\EasyAntiCheat\\x.exe", &[])));
    assert!(!never_kill(&proc("obsidian", "C:\\Program Files\\Obsidian\\obsidian.exe", &[])));
    // The Riot launcher target must not be able to reach Vanguard.
    let riot = def("riotclient");
    assert!(!matches(riot, &proc("vgc", "C:\\Program Files\\Riot Vanguard\\vgc.exe", &[])));
    assert!(!riot.procs.iter().any(|p| p.starts_with("vg")));
}

#[test]
fn docker_prefix_matches_backend_processes() {
    let docker = def("docker_wsl");
    assert!(matches(docker, &proc("docker desktop", "C:\\Program Files\\Docker\\Docker\\Docker Desktop.exe", &[])));
    assert!(matches(docker, &proc("com.docker.backend", "C:\\Program Files\\Docker\\Docker\\resources\\com.docker.backend.exe", &[])));
    assert!(!matches(docker, &proc("dockerd", "C:\\x\\dockerd.exe", &[])));
    assert!(docker.wsl_shutdown);
}

const LISTING: &str = "\
Existing Power Schemes (* Active)
-----------------------------------
Power Scheme GUID: 381b4222-f694-41f0-9685-ff5bb260df2e  (Balanced)
Power Scheme GUID: 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c  (High performance) *
Power Scheme GUID: a1841308-3541-4fab-bc81-f71556f20b4a  (Power saver (battery))
";

#[test]
fn parses_power_plans() {
    let line = "Power Scheme GUID: 29020666-3a1b-4b66-9d4c-b33d99302449  (My Custom Plan) *";
    assert_eq!(guid_in(line).as_deref(), Some("29020666-3a1b-4b66-9d4c-b33d99302449"));
    assert_eq!(plan_label(line), "My Custom Plan");
    assert_eq!(guid_in("no guid here"), None);
    assert_eq!(guid_in("Power Scheme GUID: zzzzzzzz-zzzz-zzzz-zzzz-zzzzzzzzzzzz (x)"), None);
}

#[test]
fn lists_every_installed_plan_and_marks_the_active_one() {
    let plans = parse_plans(LISTING);
    assert_eq!(plans.len(), 3, "the header lines are not plans");
    assert_eq!(plans[0].name, "Balanced");
    assert!(!plans[0].active);
    assert_eq!(plans[1].name, "High performance");
    assert!(plans[1].active);
    assert_eq!(plans[2].name, "Power saver (battery)");
    assert_eq!(plans.iter().filter(|p| p.active).count(), 1);
}

#[test]
fn only_a_real_guid_can_reach_powercfg() {
    assert_eq!(clean_guid("{381B4222-F694-41F0-9685-FF5BB260DF2E}"), "381b4222-f694-41f0-9685-ff5bb260df2e");
    assert_eq!(clean_guid(" 381b4222-f694-41f0-9685-ff5bb260df2e "), "381b4222-f694-41f0-9685-ff5bb260df2e");
    for bad in ["", "scheme_min", "381b4222-f694-41f0-9685-ff5bb260df2e; calc", "381b4222f69441f09685ff5bb260df2e0000"] {
        assert_eq!(clean_guid(bad), "", "{bad:?} must be rejected");
    }
}

#[test]
fn this_pc_reports_one_active_power_plan() {
    let plans = installed_plans();
    if !plans.is_empty() {
        assert_eq!(plans.iter().filter(|p| p.active).count(), 1);
    }
}

#[test]
fn a_scan_lists_this_pcs_apps_services_and_power_plans_without_protected_ones() {
    let report = scan();
    assert!(!report.processes.is_empty(), "a running PC has programs");
    for app in &report.processes {
        assert!(!NEVER_KILL_NAMES.contains(&app.name.as_str()), "{} must never be offered", app.name);
        assert!(!app.path.contains("\\windows\\system32\\"), "{} is a system program", app.name);
    }
    for service in &report.services {
        assert!(!service_denied(&service.name), "{} must never be offered", service.name);
    }
    if !report.power_plans.is_empty() {
        assert_eq!(report.power_plans.iter().filter(|p| p.active).count(), 1);
        assert!(report.power_plans.iter().any(|p| p.name == report.power_plan));
    }
    println!(
        "open apps: {:?}",
        report.processes.iter().filter(|p| p.has_window).map(|p| p.name.as_str()).collect::<Vec<_>>()
    );
    println!(
        "power plans: {:?}",
        report.power_plans.iter().map(|p| format!("{}{}", p.name, if p.active { " *" } else { "" })).collect::<Vec<_>>()
    );
}

#[test]
fn a_missing_or_unset_power_plan_changes_nothing() {
    let mut notes = Vec::new();
    let (_, previous, applied) = apply_plan("", &mut notes);
    assert!(previous.is_empty() && applied.is_empty() && notes.is_empty());
    let (_, previous, applied) = apply_plan("00000000-0000-0000-0000-000000000000", &mut notes);
    assert!(previous.is_empty() && applied.is_empty());
    assert_eq!(notes.len(), 1, "an uninstalled plan is explained, not applied");
    // Restoring when nothing was switched is a no-op.
    let mut notes = Vec::new();
    restore_plan("", "", &mut notes);
    assert!(notes.is_empty());
}

#[test]
fn helper_script_is_resident_limited_to_its_list_and_restores_when_deckforge_exits() {
    let services = vec!["MyDatabase".to_string(), "Some Service".to_string()];
    let script = helper_script(&services, 4242);
    assert!(script.contains("$allowed = @('MyDatabase','Some Service')"));
    assert!(script.contains("$parentPid = 4242"));
    assert!(script.contains("game-mode-cmd.json"));
    assert!(script.contains("Restore-All 'restore'"), "restores when the parent dies");
    assert!(script.contains("'vgc'") && script.contains("'audiosrv'") && script.contains("'windefend'"));
    assert!(script.contains("BelowNormal"), "the helper must not compete with the game");
    assert!(!script.contains("Set-ExecutionPolicy"));
    assert!(!script.contains("{{") && !script.contains("}}"), "format escapes were left in the script");
    assert!(!encode_powershell(&script).is_empty());
    assert_eq!(ps_quote("a'b"), "'a''b'");
}

#[test]
fn helper_script_is_valid_powershell() {
    let path = std::env::temp_dir().join("deckforge-helper-check.ps1");
    std::fs::write(&path, helper_script(&["MyDatabase".to_string()], 1)).unwrap();
    let check = format!(
        "$e=$null; [void][System.Management.Automation.Language.Parser]::ParseFile('{}',[ref]$null,[ref]$e); if ($e.Count) {{ $e | ForEach-Object {{ $_.Message }}; exit 1 }}",
        path.display()
    );
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &check])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    let _ = std::fs::remove_file(&path);
    if let Ok(output) = output {
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stdout));
    }
}

#[test]
fn helper_covers_checks_names_case_insensitively() {
    let info = HelperInfo { names: vec!["MyDatabase".into(), "Spooler".into()], ..HelperInfo::default() };
    assert!(helper_covers(&info, &["mydatabase".to_string()]));
    assert!(!helper_covers(&info, &["mydatabase".to_string(), "othersvc".to_string()]));
}

#[test]
fn every_builtin_service_passes_the_deny_list() {
    for t in TARGETS {
        for name in t.services {
            assert!(!service_denied(name), "{} lists the protected service {name}", t.id);
        }
    }
    for name in UPDATE_SERVICES {
        assert!(!service_denied(name), "update service {name} is denied");
    }
}

#[test]
fn game_paths_cover_common_launchers() {
    let hit = |p: &str| GAME_PATHS.iter().any(|f| p.to_lowercase().contains(f));
    assert!(hit("D:\\SteamLibrary\\steamapps\\common\\SomeGame\\game\\bin\\win64\\game.exe"));
    assert!(hit("G:\\Riot Games\\VALORANT\\live\\ShooterGame\\Binaries\\Win64\\VALORANT-Win64-Shipping.exe"));
    assert!(!hit("C:\\Program Files\\Mozilla Firefox\\firefox.exe"));
    assert!(NOT_GAMES.contains(&"firefox"));
}
