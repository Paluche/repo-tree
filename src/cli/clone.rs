//! Clone a repository into the repo_tree.
use clap::Args;

use crate::{
    Config, Host, UrlParser, VersionControlSystem, get_repo_tree_dir, git,
    jujutsu, repository,
};

/// Clone a repository within the repo tree.
#[derive(Args, Debug, PartialEq)]
pub struct CloneArgs {
    /// Url of the repository to clone.
    url: String,
    /// Type of version control system to use to clone the repository.
    #[arg(long, short)]
    vcs: Option<VersionControlSystem>,
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
        if let Some((current_vcs, is_git_submodule, is_jj_workspace, _)) =
            VersionControlSystem::try_new(&location)
        {
            assert!(
                !is_jj_workspace,
                "Unexpected jj workspace at the clone location"
            );
            assert!(
                !is_git_submodule,
                "Unexpected git submodule at the clone location"
            );
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
        let res = match vcs {
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
        };
        if res == 0 {
            println!("{}", location.display());
        }
        res
    }
}

pub fn run(args: CloneArgs) -> i32 {
    let config = Config::default();
    let url_parser = UrlParser::new(&config);
    let parsed_url = url_parser.parse_url(&args.url);

    if let Some((host, name)) = parsed_url {
        if let Some(host) = host {
            do_clone(args.url, host, name, args.vcs.unwrap_or(config.vcs))
        } else {
            eprintln!("Unknown host");
            1
        }
    } else {
        eprintln!("Error parsing the provided URL");
        1
    }
}
