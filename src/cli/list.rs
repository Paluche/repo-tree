use crate::{Config, UrlParser, get_workspace_dir, load_workspace};

pub fn list() -> i32 {
    let repositories = load_workspace(
        &get_workspace_dir(),
        &UrlParser::new(&Config::default()),
    )
    .0;

    for repository in repositories {
        println!("{}", repository.root.display());
    }
    0
}
