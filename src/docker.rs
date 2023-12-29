use std::collections::{BTreeMap, HashMap};

use zellij_tile::prelude::*;

pub fn request_docker_containers() {
    let context: BTreeMap<String, String> =
        BTreeMap::from([("command".to_owned(), "ps".to_owned())]);

    run_command(&["docker", "ps", "--format", "json"], context);
}

pub fn parse_docker_containers(output: &str) -> Vec<String> {
    let mut containers = vec![];
    for line in output.lines() {
        let container: HashMap<String, String> = serde_json::from_str(line).unwrap();
        containers.push(container["Names"].to_owned());
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
