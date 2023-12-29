use zellij_tile::prelude::*;

use std::collections::BTreeMap;

mod docker;

#[derive(Default)]
struct State {
    error_message: Option<String>,
    init: bool,
    containers: Vec<String>,
    containers_loading: bool,
    userspace_configuration: BTreeMap<String, String>,
}

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        self.userspace_configuration = configuration;

        self.containers = vec![];
        self.containers_loading = false;
        self.init = false;

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
            Event::RunCommandResult(status_code, stdout, stderr, _context) => {
                if status_code != Some(0) {
                    self.error_message = Some(String::from_utf8(stderr).unwrap());
                    return true;
                }

                self.containers =
                    docker::parse_docker_containers(std::str::from_utf8(&stdout).unwrap());
                self.containers_loading = false;
                self.error_message = None;
                should_render = true;
            }
            Event::Key(key) => match key {
                Key::Char('q') => {
                    close_focus();
                }
                Key::Char('r') => {
                    docker::request_docker_containers();
                    self.containers_loading = true;
                }
                _ => (),
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

        if !self.init && !self.containers_loading {
            eprintln!("Loading containers...");
            docker::request_docker_containers();
            self.containers_loading = true;
            self.init = true;
        }

        let items: Vec<NestedListItem> = self.containers.iter().map(NestedListItem::new).collect();

        print_text_with_coordinates(
            Text::new(format!("Containers ({})", self.containers.len())),
            0,
            0,
            None,
            None,
        );
        print_nested_list_with_coordinates(items, 1, 1, None, None);
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
            key: String::from("q"),
            description: String::from("Quit"),
        },
        KeyBindHelp {
            key: String::from("r"),
            description: String::from("Refresh"),
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
