use std::sync::LazyLock;

use serde::Deserialize;

const DESKTOP_LAUNCH_TOML: &str = include_str!("../../../../resources/desktop-launch.toml");

#[derive(Debug, Deserialize)]
struct DesktopLaunchTemplates {
    macos: MacosTemplates,
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    windows: WindowsTemplates,
}

#[derive(Debug, Deserialize)]
struct MacosTemplates {
    app_probe: String,
}

#[derive(Debug, Deserialize)]
struct WindowsTemplates {
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    process_probe: String,
}

static TEMPLATES: LazyLock<DesktopLaunchTemplates> = LazyLock::new(|| {
    toml::from_str(DESKTOP_LAUNCH_TOML).expect("Failed to parse desktop-launch.toml")
});

pub(super) fn macos_app_probe_script(command: &str, app_script: &str) -> String {
    render_template(
        &TEMPLATES.macos.app_probe,
        &[("command", command), ("app_script", app_script)],
    )
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(super) fn windows_process_probe_script(process_name: &str) -> String {
    render_template(
        &TEMPLATES.windows.process_probe,
        &[("process_name", process_name)],
    )
}

fn render_template(template: &str, replacements: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (key, value) in replacements {
        out = out.replace(&format!("{{{key}}}"), value);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macos_template_inserts_command_and_app_probe() {
        let script = macos_app_probe_script("open -a Codex", "'application \"Codex\" is running'");

        assert!(script.contains("open -a Codex\nstatus=$?"));
        assert!(script.contains("osascript -e 'application \"Codex\" is running'"));
        assert!(script.contains("exit \"$status\""));
    }

    #[test]
    fn windows_template_inserts_process_name() {
        let script = windows_process_probe_script("'Codex'");

        assert!(script.contains("Get-Process -Name 'Codex'"));
        assert!(script.contains("Start-Sleep -Milliseconds 500"));
    }
}
