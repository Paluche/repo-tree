use crate::load_workspace;

pub fn resolve(repo_id: String) {
    let _ = repo_id;
    for repository in load_workspace() {
        println!("{repository}");
    }
}
