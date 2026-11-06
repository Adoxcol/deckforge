use super::*;

// ---------------------------------------------------------------------------
// Services (elevated helper)
// ---------------------------------------------------------------------------

/// Every Win32 service as (name, display name, running), read once from `sc query`.
pub(super) fn service_list() -> Vec<(String, String, bool)> {
    let out = Command::new("sc.exe")
        .args(["query", "type=", "service", "state=", "all"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    let mut list = Vec::new();
    let mut name: Option<String> = None;
    let mut display = String::new();
    for line in out.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("SERVICE_NAME:") {
            name = Some(value.trim().to_string());
            display.clear();
        } else if let Some(value) = line.strip_prefix("DISPLAY_NAME:") {
            display = value.trim().to_string();
        } else if line.starts_with("STATE") {
            if let Some(found) = name.take() {
                let shown = if display.is_empty() { found.clone() } else { display.clone() };
                list.push((found, shown, line.contains("RUNNING")));
            }
        }
    }
    list
}

pub(super) fn service_states() -> HashMap<String, bool> {
    service_list()
        .into_iter()
        .map(|(name, _, running)| (name.to_lowercase(), running))
        .collect()
}

pub(super) fn ps_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// Quoted PowerShell string list, for example `'a','b'`.
pub(super) fn ps_list(items: &[&str]) -> String {
    items.iter().map(|s| ps_quote(s)).collect::<Vec<_>>().join(",")
}

/// The script the elevated helper runs. It stays resident until DeckForge exits, waits for
/// `game-mode-cmd.json`, and only ever touches the service names baked in below.
pub(super) fn helper_script(services: &[String], parent: u32) -> String {
    let dir = data_dir();
    let names = services.iter().map(|s| ps_quote(s)).collect::<Vec<_>>().join(",");
    format!(
        r#"$ErrorActionPreference = 'SilentlyContinue'
Add-Type -AssemblyName System.ServiceProcess
$dir = {dir}
$out = Join-Path $dir 'game-mode-helper.json'
$cmdFile = Join-Path $dir 'game-mode-cmd.json'
$parentPid = {parent}
$allowed = @({names})
$denyExact = @({deny_exact})
$denyPrefix = @({deny_prefix})
try {{ (Get-Process -Id $PID).PriorityClass = 'BelowNormal' }} catch {{}}
$stopped = New-Object System.Collections.ArrayList
$failed = @()
$done = ''
function Test-Denied([string]$n) {{
  $l = $n.ToLower()
  if ($denyExact -contains $l) {{ return $true }}
  foreach ($p in $denyPrefix) {{ if ($l.StartsWith($p)) {{ return $true }} }}
  return $false
}}
function Save-State {{
  $o = [pscustomobject]@{{ armed = $true; pid = $PID; names = $allowed; done = $done; stopped = @($stopped); failed = @($failed) }}
  Set-Content -Path $out -Value ($o | ConvertTo-Json -Compress) -Encoding UTF8
}}
function Stop-One([string]$n) {{
  try {{ $sc = New-Object System.ServiceProcess.ServiceController($n); $sc.Stop() }} catch {{ try {{ Stop-Service -Name $n -Force -ErrorAction Stop }} catch {{}} }}
  $until = (Get-Date).AddSeconds(25)
  while ((Get-Date) -lt $until) {{
    $s = Get-Service -Name $n -ErrorAction SilentlyContinue
    if (-not $s -or $s.Status -eq 'Stopped') {{ return $true }}
    Start-Sleep -Milliseconds 300
  }}
  return $false
}}
function Restore-All([string]$mode) {{
  if ($mode -ne 'skip') {{
    $list = @($stopped)
    [array]::Reverse($list)
    foreach ($n in $list) {{ try {{ Start-Service -Name $n -ErrorAction Stop }} catch {{}}; Start-Sleep -Milliseconds 600 }}
  }}
  $stopped.Clear()
}}
Save-State
while ($true) {{
  if (-not (Get-Process -Id $parentPid -ErrorAction SilentlyContinue)) {{
    Restore-All 'restore'
    Remove-Item $out -Force -ErrorAction SilentlyContinue
    break
  }}
  if (Test-Path $cmdFile) {{
    $raw = Get-Content $cmdFile -Raw
    Remove-Item $cmdFile -Force -ErrorAction SilentlyContinue
    $c = $null
    try {{ $c = $raw | ConvertFrom-Json }} catch {{}}
    if ($c) {{
      if ($c.cmd -eq 'stop') {{
        $failed = @()
        foreach ($n in @($c.services)) {{
          if (-not ($allowed -contains $n) -or (Test-Denied $n)) {{ continue }}
          $s = Get-Service -Name $n -ErrorAction SilentlyContinue
          if (-not $s -or $s.Status -ne 'Running') {{ continue }}
          if (Stop-One $n) {{ [void]$stopped.Add($n) }} else {{ $failed += $n }}
          Start-Sleep -Milliseconds 700
        }}
      }} elseif ($c.cmd -eq 'restore') {{
        Restore-All ([string]$c.mode)
      }} elseif ($c.cmd -eq 'quit') {{
        Restore-All 'restore'
        Remove-Item $out -Force -ErrorAction SilentlyContinue
        break
      }}
      $done = [string]$c.id
      Save-State
    }}
  }}
  Start-Sleep -Milliseconds 500
}}
"#,
        dir = ps_quote(&dir.to_string_lossy()),
        parent = parent,
        names = names,
        deny_exact = ps_list(SERVICE_DENY_EXACT),
        deny_prefix = ps_list(SERVICE_DENY_PREFIX),
    )
}

pub(super) fn encode_powershell(script: &str) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let bytes: Vec<u8> = script.encode_utf16().flat_map(|unit| unit.to_le_bytes()).collect();
    STANDARD.encode(bytes)
}

pub(super) fn read_helper_json() -> Option<serde_json::Value> {
    let raw = std::fs::read_to_string(helper_json_path()).ok()?;
    serde_json::from_str(raw.trim_start_matches('\u{feff}').trim()).ok()
}

pub(super) fn json_names(value: &serde_json::Value, key: &str) -> Vec<String> {
    match &value[key] {
        serde_json::Value::Array(items) => items.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
        serde_json::Value::String(one) => vec![one.clone()],
        _ => Vec::new(),
    }
}

/// What the running admin helper last reported.
#[derive(Debug, Clone, Default)]
pub(super) struct HelperInfo {
    pub(super) names: Vec<String>,
    pub(super) done: String,
    pub(super) stopped: Vec<String>,
    pub(super) failed: Vec<String>,
}

/// The helper, when it has started and its process is still alive.
pub(super) fn helper_info() -> Option<HelperInfo> {
    let json = read_helper_json()?;
    if json["armed"].as_bool() != Some(true) {
        return None;
    }
    let pid = json["pid"].as_u64()? as u32;
    if !pid_alive(pid) {
        return None;
    }
    Some(HelperInfo {
        names: json_names(&json, "names"),
        done: json["done"].as_str().unwrap_or("").to_string(),
        stopped: json_names(&json, "stopped"),
        failed: json_names(&json, "failed"),
    })
}

/// Every service any target (built-in or yours) or the update tweak can stop. The helper
/// gets this list when it starts and refuses anything else.
pub(super) fn helper_names(cfg: &GameModeConfig) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for def in all_targets(cfg) {
        names.extend(def.services.iter().map(|s| (*s).to_string()));
    }
    names.extend(UPDATE_SERVICES.iter().map(|s| (*s).to_string()));
    names.sort_by_key(|n| n.to_lowercase());
    names.dedup_by_key(|n| n.to_lowercase());
    names
}

/// True when the running helper already covers every service in `wanted`.
pub(super) fn helper_covers(info: &HelperInfo, wanted: &[String]) -> bool {
    let have: HashSet<String> = info.names.iter().map(|n| n.to_lowercase()).collect();
    wanted.iter().all(|n| have.contains(&n.to_lowercase()))
}

/// Starts the admin helper. When DeckForge is not elevated Windows shows one prompt, so this
/// must only run while the user is at the desktop, never during a full-screen game.
pub(super) fn arm_helper(cfg: &GameModeConfig) -> Result<(), String> {
    let wanted = helper_names(cfg);
    if let Some(info) = helper_info() {
        if helper_covers(&info, &wanted) {
            return Ok(());
        }
        // The service list grew since it started. Replace it, which restores nothing in
        // between because Game Mode is not running a stop at this point.
        if !is_active() {
            if helper_send(serde_json::json!({ "cmd": "quit" })).is_ok() {
                let started = Instant::now();
                while started.elapsed() < Duration::from_secs(8) && helper_info().is_some() {
                    std::thread::sleep(Duration::from_millis(250));
                }
            }
        } else {
            return Ok(());
        }
    }
    std::fs::create_dir_all(data_dir()).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(helper_json_path());
    let _ = std::fs::remove_file(cmd_path());
    let encoded = encode_powershell(&helper_script(&wanted, std::process::id()));
    let mut command = Command::new("powershell.exe");
    command.creation_flags(CREATE_NO_WINDOW);
    if is_admin() {
        command.args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-EncodedCommand",
            &encoded,
        ]);
    } else {
        let launcher = format!(
            "try {{ Start-Process -FilePath 'powershell.exe' -Verb RunAs -WindowStyle Hidden -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-EncodedCommand','{encoded}' -ErrorAction Stop }} catch {{ exit 3 }}"
        );
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &launcher,
        ]);
    }
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let started = Instant::now();
    loop {
        if helper_info().is_some() {
            return Ok(());
        }
        if let Ok(Some(exit)) = child.try_wait() {
            if !exit.success() && helper_info().is_none() {
                return Err("The Windows permission prompt was declined, so services were left running.".to_string());
            }
        }
        if started.elapsed() > Duration::from_secs(90) {
            return Err("The admin helper did not start in time, so services were left running.".to_string());
        }
        std::thread::sleep(Duration::from_millis(300));
    }
}

pub(super) static COMMAND_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Hands the helper one command and returns its id.
pub(super) fn helper_send(mut command: serde_json::Value) -> Result<String, String> {
    let id = format!("{}-{}", now_ms(), COMMAND_COUNTER.fetch_add(1, Ordering::SeqCst));
    command["id"] = serde_json::Value::String(id.clone());
    let raw = serde_json::to_string(&command).map_err(|e| e.to_string())?;
    let temp = data_dir().join("game-mode-cmd.tmp");
    std::fs::write(&temp, raw).map_err(|e| e.to_string())?;
    std::fs::rename(&temp, cmd_path()).map_err(|e| e.to_string())?;
    Ok(id)
}

/// Waits until the helper reports it finished the command with this id.
pub(super) fn helper_wait(id: &str, limit: Duration) -> Result<HelperInfo, String> {
    let started = Instant::now();
    loop {
        match helper_info() {
            Some(info) if info.done == id => return Ok(info),
            None => return Err("The admin helper stopped running.".to_string()),
            _ => {}
        }
        if started.elapsed() > limit {
            return Err("The admin helper did not report back in time.".to_string());
        }
        std::thread::sleep(Duration::from_millis(300));
    }
}

