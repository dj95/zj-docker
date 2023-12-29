use std::collections::{BTreeMap, HashMap};

use zellij_tile::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Container {
    pub name: String,
    pub running: bool,
}

impl Default for Container {
    fn default() -> Self {
        Self {
            name: "".to_owned(),
            running: false,
        }
    }
}

pub fn request_docker_containers() {
    let context: BTreeMap<String, String> =
        BTreeMap::from([("command".to_owned(), "ps".to_owned())]);

    run_command(&["docker", "ps", "--format", "json", "-a"], context);
}

pub fn parse_docker_containers(output: &str) -> Vec<Container> {
    let mut containers = vec![];
    for line in output.lines() {
        let container: HashMap<String, String> = serde_json::from_str(line).unwrap();
        containers.push(Container {
            name: container["Names"].to_owned(),
            running: container["State"] == "running",
        });
    }
    containers
}

pub fn open_container(container: &str) {
    open_command_pane(CommandToRun::new_with_args(
        "docker",
        vec!["logs", "-f", container],
    ));
}

pub fn close_container(container: &str) {
    let context: BTreeMap<String, String> =
        BTreeMap::from([("command".to_owned(), "stop".to_owned())]);
    run_command(&["docker", "stop", container], context);
}
