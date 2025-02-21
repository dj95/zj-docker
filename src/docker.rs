use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashMap},
};

use zellij_tile::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Container {
    pub id: String,
    pub name: String,
    pub image: String,
    pub running: bool,
    pub status: String,
}

impl ToString for Container {
    fn to_string(&self) -> String {
        format!(
            "{} {}     {} ({})",
            self.id, self.name, self.image, self.status
        )
    }
}

impl Container {
    pub fn to_table_row(&self) -> Vec<Text> {
        vec![
            Text::new(self.id.clone()),
            Text::new(" "),
            Text::new(self.name.clone()),
            Text::new(" "),
            Text::new(self.image.clone()),
            Text::new(" "),
            Text::new(self.status.clone()),
        ]
    }
}

impl Default for Container {
    fn default() -> Self {
        Self {
            id: "".to_owned(),
            name: "".to_owned(),
            image: "".to_owned(),
            running: false,
            status: "".to_owned(),
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
            id: container["ID"].to_owned(),
            name: container["Names"].to_owned(),
            image: container["Image"].to_owned(),
            running: container["State"] == "running",
            status: container["Status"].to_owned(),
        });
    }

    containers.sort_by(|a, b| {
        if a.running == b.running {
            return Ordering::Equal;
        }

        if a.running && !b.running {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    });

    containers
}

pub fn open_container(container: &str) {
    open_command_pane(
        CommandToRun::new_with_args("docker", vec!["logs", "-f", container]),
        BTreeMap::new(),
    );
}

pub fn start_container(container: &str) {
    let context: BTreeMap<String, String> =
        BTreeMap::from([("command".to_owned(), "start".to_owned())]);
    run_command(&["docker", "start", container], context);
}

pub fn delete_container(container: &str) {
    let context: BTreeMap<String, String> =
        BTreeMap::from([("command".to_owned(), "delete".to_owned())]);
    run_command(&["docker", "rm", container], context);
}

pub fn close_container(container: &str) {
    let context: BTreeMap<String, String> =
        BTreeMap::from([("command".to_owned(), "stop".to_owned())]);
    run_command(&["docker", "stop", container], context);
}
