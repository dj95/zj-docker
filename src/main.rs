use rust_fuzzy_search::fuzzy_search;
use zellij_tile::prelude::*;

use docker::Container;
use std::collections::BTreeMap;

mod docker;

#[derive(Default)]
struct State {
    error_message: Option<String>,
    init: bool,
    containers: Vec<Container>,
    filtered_containers: Vec<Container>,
    search_query: String,
    containers_loading: bool,
    selected_container: Option<String>,
    userspace_configuration: BTreeMap<String, String>,
}

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        self.userspace_configuration = configuration;

        self.containers = vec![];
        self.containers_loading = false;
        self.init = false;
        self.search_query = "".to_owned();
        self.filtered_containers = vec![];

        // we need the ReadApplicationState permission to receive the ModeUpdate and TabUpdate
        // events
        // we need the RunCommands permission to run "cargo test" in a floating window
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::RunCommands,
            PermissionType::ChangeApplicationState,
        ]);

        subscribe(&[
            EventType::ModeUpdate,
            EventType::TabUpdate,
            EventType::Key,
            EventType::RunCommandResult,
        ]);
    }

    fn update(&mut self, event: Event) -> bool {
        let mut should_render = false;
        match event {
            Event::RunCommandResult(status_code, stdout, stderr, context) => {
                if status_code != Some(0) {
                    self.error_message = Some(String::from_utf8(stderr).unwrap());
                    return true;
                }

                if context.get("command") == Some(&"start".to_owned()) {
                    docker::request_docker_containers();
                    return false;
                }
                if context.get("command") == Some(&"stop".to_owned()) {
                    docker::request_docker_containers();
                    return false;
                }
                if context.get("command") == Some(&"delete".to_owned()) {
                    docker::request_docker_containers();
                    return false;
                }
                if context.get("command") != Some(&"ps".to_owned()) {
                    return false;
                }

                self.containers =
                    docker::parse_docker_containers(std::str::from_utf8(&stdout).unwrap());
                self.containers_loading = false;
                self.error_message = None;
                self.selected_container = None;

                should_render = true;
            }
            Event::Key(key) => match key.bare_key {
                BareKey::Up => {
                    let container_index = self
                        .filtered_containers
                        .iter()
                        .position(|c| Some(c.name.clone()) == self.selected_container)
                        .unwrap_or(0);

                    if container_index == 0 {
                        return false;
                    }

                    self.selected_container = Some(
                        self.filtered_containers
                            .get(container_index - 1)
                            .unwrap_or(&Default::default())
                            .to_owned()
                            .name,
                    );

                    should_render = true;
                }
                BareKey::Down => {
                    let container_index = self
                        .filtered_containers
                        .iter()
                        .position(|c| Some(c.name.clone()) == self.selected_container)
                        .unwrap_or(0);

                    if self.filtered_containers.is_empty()
                        || container_index == self.filtered_containers.len() - 1
                    {
                        return false;
                    }

                    self.selected_container = Some(
                        self.filtered_containers
                            .get(container_index + 1)
                            .unwrap_or(&Default::default())
                            .to_owned()
                            .name,
                    );

                    should_render = true;
                }
                BareKey::Char('r') if key.has_modifiers(&[KeyModifier::Ctrl]) => {
                    docker::request_docker_containers();
                    self.containers_loading = true;
                }
                BareKey::Char('e') if key.has_modifiers(&[KeyModifier::Ctrl]) => {
                    if let Some(ref container) = self.selected_container {
                        docker::start_container(container);
                    }
                }
                BareKey::Char('d') if key.has_modifiers(&[KeyModifier::Ctrl]) => {
                    if let Some(ref container) = self.selected_container {
                        docker::delete_container(container);
                    }
                }
                BareKey::Char('c') if key.has_modifiers(&[KeyModifier::Ctrl]) => {
                    if let Some(ref container) = self.selected_container {
                        docker::close_container(container);
                    }
                }
                BareKey::Backspace => {
                    if self.search_query.is_empty() {
                        return false;
                    }

                    self.search_query = self
                        .search_query
                        .chars()
                        .take(self.search_query.len() - 1)
                        .collect();

                    should_render = true;
                }
                BareKey::Enter => {
                    if let Some(ref container) = self.selected_container {
                        docker::open_container(container);
                    }
                }
                BareKey::Char(c) => match c {
                    _ => {
                        self.search_query = self.search_query.clone() + &c.to_string();
                        should_render = true;
                    }
                },
                BareKey::Esc => {
                    close_self();
                }
                _ => {
                    eprintln!("Key pressed: {:?}", key);
                }
            },
            _ => (),
        };

        should_render
    }

    fn render(&mut self, rows: usize, cols: usize) {
        if let Some(error_message) = &self.error_message {
            print_text_with_coordinates(
                Text::new(format!("Error: {error_message}")).color_range(0, 0..cols),
                0,
                rows / 2,
                None,
                None,
            );

            return;
        }

        self.filtered_containers = filtered_containers(self.containers.clone(), &self.search_query);

        if !self.init && !self.containers_loading {
            docker::request_docker_containers();
            self.containers_loading = true;
            self.init = true;
        }

        self.selected_container =
            get_selected_container(&self.selected_container, &self.filtered_containers);

        print_text_with_coordinates(
            Text::new(format!("Search > {}│", self.search_query)),
            0,
            0,
            None,
            None,
        );

        let (running_items, running_items_len) =
            get_running_table_with_size(&self.filtered_containers, &self.selected_container, true);
        print_text_with_coordinates(
            Text::new(format!("Containers ({})", running_items_len)),
            0,
            2,
            None,
            None,
        );
        print_table_with_coordinates(running_items, 2, 3, None, None);

        let (stopped_items, stopped_items_len) =
            get_running_table_with_size(&self.filtered_containers, &self.selected_container, false);
        print_text_with_coordinates(
            Text::new(format!("Stopped Containers ({})", stopped_items_len)),
            0,
            4 + running_items_len + 1,
            None,
            None,
        );
        print_table_with_coordinates(stopped_items, 2, 5 + running_items_len + 1, None, None);

        print_help(rows);
    }
}

fn get_running_table_with_size(
    filtered_containers: &Vec<Container>,
    selected_container: &Option<String>,
    active: bool,
) -> (Table, usize) {
    let mut running_items = Table::new();
    let mut running_items_len = 0;

    running_items =
        running_items.add_row(["ID", " ", "Name", " ", "Image", " ", "Status"].to_vec());

    for container in filtered_containers {
        if container.running != active {
            continue;
        }

        let mut row = container.to_table_row();

        if Some(container.name.clone()) == *selected_container {
            row = row.iter().map(|t| t.clone().selected()).collect();
        }

        running_items = running_items.add_styled_row(row);
        running_items_len += 1;
    }

    (running_items, running_items_len)
}

fn get_selected_container(
    selected_container: &Option<String>,
    filtered_containers: &[Container],
) -> Option<String> {
    if selected_container.is_none() && !filtered_containers.is_empty() {
        return filtered_containers.first().map(|c| c.name.clone());
    }

    if !filtered_containers
        .iter()
        .any(|c| Some(c.name.clone()) == *selected_container)
    {
        return filtered_containers.first().map(|c| c.name.clone());
    }

    selected_container.clone()
}

fn filtered_containers(containers: Vec<Container>, search_query: &str) -> Vec<Container> {
    if search_query.is_empty() {
        return containers;
    }

    let mapped_containers = containers
        .iter()
        .map(|c| c.name.as_ref())
        .collect::<Vec<&str>>();

    let mut filtered_containers = fuzzy_search(search_query, &mapped_containers);

    filtered_containers.sort_by(|(_, a_score), (_, b_score)| {
        a_score
            .partial_cmp(b_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    filtered_containers.reverse();

    let filtered_containers = filtered_containers
        .iter()
        .filter(|(_, score)| *score > 0.0)
        .map(|(c, _)| c.to_string())
        .collect::<Vec<String>>();

    containers
        .clone()
        .into_iter()
        .filter(|c| filtered_containers.contains(&c.name))
        .collect()
}

struct KeyBindHelp {
    key: String,
    description: String,
}

fn print_help(rows: usize) {
    let prefix = "Help: ";
    let bindings = [
        KeyBindHelp {
            key: String::from("Ctrl-r"),
            description: String::from("Refresh"),
        },
        KeyBindHelp {
            key: String::from("Enter"),
            description: String::from("Follow Logs"),
        },
        KeyBindHelp {
            key: String::from("Ctrl-c"),
            description: String::from("Stop"),
        },
        KeyBindHelp {
            key: String::from("Ctrl-e"),
            description: String::from("Start"),
        },
        KeyBindHelp {
            key: String::from("Ctrl-d"),
            description: String::from("Delete"),
        },
        KeyBindHelp {
            key: String::from("Esc"),
            description: String::from("Close"),
        },
    ];

    let mut color_ranges: Vec<_> = vec![];
    let mut pos: usize = prefix.len();

    let output = bindings
        .iter()
        .map(|kb| {
            let res = format!("<{}> => {}. ", kb.key, kb.description);
            color_ranges.push(pos + 1..=pos + kb.key.len());
            pos += res.len();
            res
        })
        .collect::<Vec<String>>()
        .join("");

    let mut text = Text::new(format!("{prefix}{}", output));

    for cr in color_ranges {
        text = text.color_range(2, cr);
    }

    print_text_with_coordinates(text, 0, rows, None, None)
}
