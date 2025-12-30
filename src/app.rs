use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct App {
    pub name: String,
    pub description: String,
    pub desktop_file: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CustomCommand {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub subcommands: Vec<SubCommand>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubCommand {
    pub name: String,
    pub command: String,
}

#[derive(Clone, Debug)]
pub enum SearchItem {
    App(App),
    CustomCommand(CustomCommand),
    SubCommand { parent: String, sub: SubCommand },
}

impl SearchItem {
    pub fn name(&self) -> &str {
        match self {
            Self::App(app) => &app.name,
            Self::CustomCommand(cmd) => &cmd.name,
            Self::SubCommand { sub, .. } => &sub.name,
        }
    }

    pub fn description(&self) -> String {
        match self {
            Self::App(app) => app.description.clone(),
            Self::CustomCommand(cmd) => cmd.description.clone(),
            Self::SubCommand { parent, .. } => parent.clone(),
        }
    }

    pub fn execute(&self) {
        match self {
            Self::App(app) => launch_app(&app.desktop_file),
            Self::CustomCommand(cmd) => {
                if let Some(command) = &cmd.command {
                    execute_command(command);
                }
            }
            Self::SubCommand { sub, .. } => execute_command(&sub.command),
        }
    }
}

fn launch_app(desktop_file: &str) {
    let desktop_file = desktop_file.to_string();
    tokio::spawn(async move {
        let _ = tokio::process::Command::new("gio")
            .arg("launch")
            .arg(&desktop_file)
            .spawn();
    });
}

fn execute_command(command: &str) {
    let command = command.to_string();
    tokio::spawn(async move {
        let _ = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&command)
            .spawn();
    });
}

#[derive(Clone, Debug, PartialEq)]
pub enum FilterMode {
    All,
    Apps,
    Commands,
}

pub struct State {
    apps: Vec<App>,
    commands: Vec<CustomCommand>,
    matcher: SkimMatcherV2,
}

impl State {
    pub fn new() -> Self {
        Self {
            apps: Vec::new(),
            commands: Vec::new(),
            matcher: SkimMatcherV2::default(),
        }
    }

    pub async fn load(&mut self) {
        self.apps = crate::config::load_apps().await;
        self.commands = crate::config::load_commands().await;
    }

    pub fn search(&self, query: &str, mode: &FilterMode) -> Vec<SearchItem> {
        if query.is_empty() {
            return Vec::new();
        }

        // Handle subcommand queries
        if let Some(parent_end) = query.find(" > ") {
            return self.search_subcommands(query, parent_end);
        }

        // Fuzzy search
        let mut scored: Vec<(SearchItem, i64)> = match mode {
            FilterMode::All => {
                let mut results = self.search_commands(query);
                results.extend(self.search_apps(query));
                results
            }
            FilterMode::Apps => self.search_apps(query),
            FilterMode::Commands => self.search_commands(query),
        };

        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored.into_iter().take(10).map(|(item, _)| item).collect()
    }

    fn search_subcommands(&self, query: &str, parent_end: usize) -> Vec<SearchItem> {
        let parent_name = &query[..parent_end];
        let sub_query = query[parent_end + 3..].trim().to_lowercase();

        self.commands.iter()
            .find(|cmd| cmd.name.eq_ignore_ascii_case(parent_name))
            .map(|cmd| {
                cmd.subcommands.iter()
                    .filter(|sub| sub_query.is_empty() ||
                        sub.name.to_lowercase().contains(&sub_query))
                    .map(|sub| SearchItem::SubCommand {
                        parent: cmd.name.clone(),
                        sub: sub.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn search_apps(&self, query: &str) -> Vec<(SearchItem, i64)> {
        self.apps.iter()
            .filter_map(|app| {
                self.matcher.fuzzy_match(&app.name, query)
                    .map(|score| (SearchItem::App(app.clone()), score))
            })
            .collect()
    }

    fn search_commands(&self, query: &str) -> Vec<(SearchItem, i64)> {
        self.commands.iter()
            .filter_map(|cmd| {
                self.matcher.fuzzy_match(&cmd.name, query)
                    .map(|score| (SearchItem::CustomCommand(cmd.clone()), score))
            })
            .collect()
    }
}