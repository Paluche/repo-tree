//! Clone a repository into the repo_tree.
use crate::{
    Config, Host, UrlParser, VersionControlSystem, get_repo_tree_dir, git,
    jujutsu, repository,
};

fn prompt_for_vcs() -> VersionControlSystem {
    panic!(
        "Not implemented yet: Prompt to select the clone method, with
        JujutsuGit as default."
    );
}

fn do_clone(
    remote_url: String,
    host: Host,
    name: String,
    vcs: VersionControlSystem,
) -> i32 {
    let repo_tree_dir = &get_repo_tree_dir();
    let location = repository::location(repo_tree_dir, &host, &name);

    if location.exists() {
        if let Some((current_vcs, _)) = VersionControlSystem::try_new(&location)
        {
            if current_vcs == vcs {
                eprintln!("{vcs} repository already cloned");
                println!("{}", location.display());
                0
            } else if matches!(current_vcs, VersionControlSystem::Git)
                && matches!(vcs, VersionControlSystem::JujutsuGit)
            {
                eprintln!("Repository already cloned, initializing JJ into");
                jujutsu::git::init_colocate(&location)
            } else {
                eprintln!(
                    "{vcs} repository already cloned but is a {current_vcs} \
                     repository"
                );
                println!("{}", location.display());
                0
            }
        } else {
            eprintln!("Clone location {} already exists", location.display());
            1
        }
    } else {
        match vcs {
            VersionControlSystem::Git => git::clone(&remote_url, &location),
            VersionControlSystem::JujutsuGit => {
                let res = git::clone(&remote_url, &location);
                if res != 0 {
                    res
                } else {
                    jujutsu::git::init_colocate(&location)
                }
            }
            VersionControlSystem::Jujutsu => {
                jujutsu::git::clone(&remote_url, &location)
            }
        }
    }
}

pub fn clone(remote_url: String, vcs: Option<VersionControlSystem>) -> i32 {
    let config = Config::default();
    let url_parser = UrlParser::new(&config);
    let parsed_url = url_parser.parse_url(&remote_url);

    if let Some((host, name)) = parsed_url {
        if let Some(host) = host {
            do_clone(remote_url, host, name, vcs.unwrap_or_else(prompt_for_vcs))
        } else {
            eprintln!("Unknown host");
            1
        }
    } else {
        eprintln!("Error parsing the provided URL");
        1
    }
}
