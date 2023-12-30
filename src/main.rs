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

                if context.get("command") == Some(&"stop".to_owned()) {
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
                should_render = true;
            }
            Event::Key(key) => match key {
                Key::Up => {
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
                Key::Down => {
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
                Key::Ctrl('r') => {
                    docker::request_docker_containers();
                    self.containers_loading = true;
                }
                Key::Ctrl('c') => {
                    if let Some(ref container) = self.selected_container {
                        docker::close_container(container);
                    }
                }
                Key::Backspace => {
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
                Key::Char(c) => match c {
                    '\n' => {
                        if let Some(ref container) = self.selected_container {
                            docker::open_container(container);
                        }
                    }
                    _ => {
                        self.search_query = self.search_query.clone() + &c.to_string();
                        should_render = true;
                    }
                },
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

        if !self.search_query.is_empty() {
            let mapped_containers = self
                .containers
                .iter()
                .map(|c| c.name.as_ref())
                .collect::<Vec<&str>>();

            let mut filtered_containers = fuzzy_search(&self.search_query, &mapped_containers);

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

            self.filtered_containers = self
                .containers
                .clone()
                .into_iter()
                .filter(|c| filtered_containers.contains(&c.name))
                .collect();

            eprintln!("Filtered containers: {:?}", filtered_containers);
        } else {
            self.filtered_containers = self.containers.clone();
        }

        if !self.init && !self.containers_loading {
            eprintln!("Loading containers...");
            docker::request_docker_containers();
            self.containers_loading = true;
            self.init = true;
        }

        let mut selected_container =
            self.selected_container
                .clone()
                .unwrap_or(match self.filtered_containers.first() {
                    Some(container) => container.name.to_owned(),
                    None => String::from(""),
                });

        if !self
            .filtered_containers
            .iter()
            .any(|c| c.name == selected_container)
        {
            selected_container = match self.filtered_containers.first().cloned() {
                Some(container) => container.name.to_owned(),
                None => String::from(""),
            };
        }

        if self.selected_container.is_none() && !selected_container.is_empty() {
            self.selected_container = Some(selected_container.clone());
        }

        eprintln!("Selected container: {:?}", self.selected_container);

        let items: Vec<NestedListItem> = self
            .filtered_containers
            .iter()
            .map(|c| {
                if *c.name == selected_container {
                    return NestedListItem::new(&c.name).selected();
                }
                NestedListItem::new(&c.name)
            })
            .collect();

        print_text_with_coordinates(
            Text::new(format!("Search > {}", self.search_query)),
            0,
            0,
            None,
            None,
        );

        print_text_with_coordinates(
            Text::new(format!("Containers ({})", self.containers.len())),
            0,
            2,
            None,
            None,
        );
        print_nested_list_with_coordinates(items, 1, 3, None, None);
        print_help(rows);
    }
}

struct KeyBindHelp {
    key: String,
    description: String,
}

fn print_help(rows: usize) {
    let prefix = "Help: ";
    let bindings = vec![
        KeyBindHelp {
            key: String::from("Ctrl-r"),
            description: String::from("Refresh"),
        },
        KeyBindHelp {
            key: String::from("Ctrl-c"),
            description: String::from("Stop"),
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
