use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame, Terminal,
};
use std::{collections::HashMap, io};
use crate::profile::Profile;

struct TreeApp {
    profiles: Vec<Profile>,
    tree_items: Vec<TreeItem>,
    list_state: ListState,
    selected_profile: Option<(String, String, String)>,
    expanded: HashMap<String, bool>,
}

#[derive(Clone)]
struct TreeItem {
    label: String,
    client: String,
    account: String,
    role: String,
    level: usize,
}

impl TreeApp {
    fn new(profiles: Vec<Profile>) -> Self {
        let mut client_map: HashMap<String, HashMap<String, Vec<String>>> = HashMap::new();
        
        for profile in &profiles {
            client_map
                .entry(profile.client.clone())
                .or_default()
                .entry(profile.account.clone())
                .or_default()
                .push(profile.role.clone());
        }

        let mut app = Self {
            profiles,
            tree_items: Vec::new(),
            list_state: ListState::default(),
            selected_profile: None,
            expanded: HashMap::new(),
        };
        
        app.rebuild_tree(&client_map);
        app.list_state.select(Some(0));
        app
    }
    
    fn rebuild_tree(&mut self, client_map: &HashMap<String, HashMap<String, Vec<String>>>) {
        self.tree_items.clear();
        
        let mut sorted_clients: Vec<_> = client_map.keys().collect();
        sorted_clients.sort();
        
        for client in sorted_clients {
            self.tree_items.push(TreeItem {
                label: format!("📁 {}", client),
                client: client.clone(),
                account: String::new(),
                role: String::new(),
                level: 0,
            });
            
            if *self.expanded.get(client).unwrap_or(&false) {
                let accounts = &client_map[client];
                let mut sorted_accounts: Vec<_> = accounts.keys().collect();
                sorted_accounts.sort();
                
                for account in sorted_accounts {
                    let account_key = format!("{}-{}", client, account);
                    self.tree_items.push(TreeItem {
                        label: format!("  📁 {}", account),
                        client: client.clone(),
                        account: account.clone(),
                        role: String::new(),
                        level: 1,
                    });
                    
                    if *self.expanded.get(&account_key).unwrap_or(&false) {
                        let roles = &accounts[account];
                        let mut sorted_roles = roles.clone();
                        sorted_roles.sort();
                        
                        for role in sorted_roles {
                            self.tree_items.push(TreeItem {
                                label: format!("    📄 {}", role),
                                client: client.clone(),
                                account: account.clone(),
                                role: role.clone(),
                                level: 2,
                            });
                        }
                    }
                }
            }
        }
    }

    fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => (i + 1) % self.tree_items.len(),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.tree_items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn select(&mut self) {
        if let Some(i) = self.list_state.selected() {
            let item = &self.tree_items[i].clone();
            
            if item.level == 2 && !item.role.is_empty() {
                self.selected_profile = Some((item.client.clone(), item.account.clone(), item.role.clone()));
            } else if item.level == 0 {
                // Toggle client expansion
                let expanded = !self.expanded.get(&item.client).unwrap_or(&false);
                self.expanded.insert(item.client.clone(), expanded);
                self.rebuild_from_profiles();
            } else if item.level == 1 {
                // Toggle account expansion
                let account_key = format!("{}-{}", item.client, item.account);
                let expanded = !self.expanded.get(&account_key).unwrap_or(&false);
                self.expanded.insert(account_key, expanded);
                self.rebuild_from_profiles();
            }
        }
    }
    
    fn rebuild_from_profiles(&mut self) {
        let mut client_map: HashMap<String, HashMap<String, Vec<String>>> = HashMap::new();
        
        for profile in &self.profiles {
            client_map
                .entry(profile.client.clone())
                .or_default()
                .entry(profile.account.clone())
                .or_default()
                .push(profile.role.clone());
        }
        
        self.rebuild_tree(&client_map);
    }
}

fn ui(f: &mut Frame, app: &mut TreeApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(f.area());

    let items: Vec<ListItem> = app
        .tree_items
        .iter()
        .map(|item| {
            let style = if item.level == 2 {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Yellow)
            };
            ListItem::new(Line::from(Span::styled(&item.label, style)))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("AWS Profiles"))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, chunks[0], &mut app.list_state);
}

pub fn tui_tree_select(profiles: &[Profile]) -> Result<Option<(String, String, String)>, Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = TreeApp::new(profiles.to_vec());

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Down => app.next(),
                KeyCode::Up => app.previous(),
                KeyCode::Enter => {
                    app.select();
                    if app.selected_profile.is_some() {
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(app.selected_profile)
}