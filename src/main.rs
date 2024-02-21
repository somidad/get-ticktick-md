use colored::*;
use regex::Regex;
use std::{
    collections::HashMap,
    fs,
    io::{self, Write},
    path::Path,
    process::exit,
};
mod api;
use api::*;
mod config;
use config::*;

fn main() {
    let url = format!("https://ticktick.com/oauth/authorize?scope=tasks:read&client_id={CLIENT_ID}&response_type=code").blue();
    println!("Visit {url} to get access token");

    print!("Enter {}: ", "code".magenta());
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let code = input.trim();

    let client = reqwest::blocking::Client::new();
    let mut oauth_request_form = HashMap::new();
    // oauth_request_form.insert("cliend_id", client_id);
    // oauth_request_form.insert("client_secret", config::CLIENT_SECRET);
    oauth_request_form.insert("code", code);
    oauth_request_form.insert("grant_type", "authorization_code");
    oauth_request_form.insert("scope", "tasks:read");

    let access_token = client
        .post("https://ticktick.com/oauth/token")
        .basic_auth(CLIENT_ID, Some(config::CLIENT_SECRET))
        .form(&oauth_request_form)
        .send()
        .unwrap()
        .json::<AccessTokenResponse>()
        .unwrap()
        .access_token;
    // println!("Access token: {access_token}");

    let project_list = client
        .get("https://ticktick.com/open/v1/project")
        .bearer_auth(&access_token)
        .send()
        .unwrap()
        .json::<Vec<ProjectInfo>>()
        .unwrap();
    if project_list.len() == 0 {
        println!("{}: ", "No list to export. Exit".red());
        exit(0);
    }
    println!("==== Lists ====");
    for i in 0..project_list.len() {
        println!("{}: {}", i, project_list[i].name);
    }
    let last_index = project_list.len() - 1;
    print!(
        "Enter index of list to export {} or {}: ",
        format!("(0-{})", last_index).magenta(),
        "'all'".magenta(),
    );
    io::stdout().flush().unwrap();
    input.clear();
    io::stdin().read_line(&mut input).unwrap();
    let stop = if input.trim() == "all" {
        project_list.len()
    } else {
        let error_msg = format!("Valid input: 0-{} or 'all'", last_index);
        let index = input.trim().parse::<usize>().expect(&error_msg);
        if index > last_index {
            panic!("{error_msg}");
        }
        index + 1
    };
    for index in 0..stop {
        export_project(&project_list[index], &access_token);
    }
    println!("{}", "Done".green());
}

fn export_project(project_info: &ProjectInfo, access_token: &str) {
    let project_id = &project_info.id;
    let project_name = &project_info.name;

    let client = reqwest::blocking::Client::new();
    let task_list = client
        .get(format!(
            "https://ticktick.com/open/v1/project/{project_id}/data"
        ))
        .bearer_auth(&access_token)
        .send()
        .unwrap()
        .json::<ProjectWithData>()
        .unwrap()
        .tasks;

    let invalid_file_chars = Regex::new(r#"[<>:"/\\|?*]"#).unwrap();

    let replaced_project_name = invalid_file_chars.replace_all(project_name, "_");
    let foldername = replaced_project_name.into_owned();
    let foldername_unique = if Path::new(&foldername).exists() {
        format!("{foldername}_{project_id}")
    } else {
        foldername
    };
    fs::create_dir(&foldername_unique).unwrap();

    for task in &task_list {
        let taskname = match &task.title {
            Some(title) => {
                let replaced_task_name = invalid_file_chars.replace_all(title, "_");
                replaced_task_name.into_owned()
            }
            None => task.id.to_owned(),
        };
        let taskname_unique =
            if Path::new(format!("{foldername_unique}/{taskname}.md").as_str()).exists() {
                format!("{taskname}_{}", task.id)
            } else {
                taskname
            };
        let mut file =
            fs::File::create(format!("{foldername_unique}/{taskname_unique}.md")).unwrap();

        file.write_all(b"---\n").unwrap();
        file.write_all(b"tags:\n").unwrap();
        match &task.tags {
            Some(tags) => {
                if tags.len() > 0 {
                    for tag in task.tags.as_ref().unwrap() {
                        file.write_all(format!("  - {tag}\n").as_bytes()).unwrap();
                    }
                }
            }
            None => {}
        }
        file.write_all(b"---\n").unwrap();

        match &task.content {
            Some(content) => {
                file.write_all(b"\n").unwrap();
                file.write_all(content.as_bytes()).unwrap();
            }
            None => {}
        }
    }
}
