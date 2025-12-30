use crate::app::{App, CustomCommand};

pub async fn load_commands() -> Vec<CustomCommand> {
    let config_dirs = vec![
        "/etc/xdg/elara-launcher/commands".to_string(),
        format!("{}/.config/elara-launcher/commands",
                std::env::var("HOME").unwrap_or_default()),
    ];

    let mut commands = Vec::new();
    for dir in config_dirs {
        commands.extend(load_commands_from_dir(&dir).await);
    }
    commands
}

async fn load_commands_from_dir(dir: &str) -> Vec<CustomCommand> {
    let mut commands = Vec::new();
    let Ok(mut entries) = tokio::fs::read_dir(dir).await else {
        return commands;
    };

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Ok(content) = tokio::fs::read_to_string(&path).await {
                if let Ok(cmd) = serde_json::from_str::<CustomCommand>(&content) {
                    commands.push(cmd);
                } else if let Ok(cmds) = serde_json::from_str::<Vec<CustomCommand>>(&content) {
                    commands.extend(cmds);
                }
            }
        }
    }
    commands
}

pub async fn load_apps() -> Vec<App> {
    let home = std::env::var("HOME").unwrap_or_default();
    let paths = vec![
        "/usr/share/applications".to_string(),
        format!("{home}/.local/share/applications"),
        "/var/lib/flatpak/exports/share/applications".to_string(),
        format!("{home}/.local/share/flatpak/exports/share/applications"),
    ];

    let handles: Vec<_> = paths.into_iter()
        .map(|p| tokio::spawn(load_apps_from_dir(p)))
        .collect();

    let mut apps = Vec::new();
    for handle in handles {
        if let Ok(dir_apps) = handle.await {
            apps.extend(dir_apps);
        }
    }
    apps
}

async fn load_apps_from_dir(path: String) -> Vec<App> {
    let Ok(mut read_dir) = tokio::fs::read_dir(&path).await else {
        return Vec::new();
    };

    let mut entries = Vec::new();
    while let Ok(Some(entry)) = read_dir.next_entry().await {
        if entry.path().extension().and_then(|s| s.to_str()) == Some("desktop") {
            entries.push(entry.path());
        }
    }

    let handles: Vec<_> = entries.into_iter()
        .map(|p| tokio::spawn(parse_desktop_file(p)))
        .collect();

    let mut apps = Vec::new();
    for handle in handles {
        if let Ok(Some(app)) = handle.await {
            apps.push(app);
        }
    }
    apps
}

async fn parse_desktop_file(path: std::path::PathBuf) -> Option<App> {
    let content = tokio::fs::read_to_string(&path).await.ok()?;

    let mut name = None;
    let mut description = String::new();

    for line in content.lines() {
        match line {
            l if l.starts_with("Name=") && name.is_none() =>
                name = Some(l.trim_start_matches("Name=").to_string()),
            l if l.starts_with("Comment=") && description.is_empty() =>
                description = l.trim_start_matches("Comment=").to_string(),
            "Hidden=true" | "NoDisplay=true" => return None,
            l if l.starts_with("OnlyShowIn=") => return None,
            _ => {}
        }
    }

    Some(App {
        name: name?,
        description,
        desktop_file: path.to_string_lossy().to_string(),
    })
}