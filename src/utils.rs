use std::path::PathBuf;

pub fn find_project_root() -> Option<PathBuf> {
    let mut current = std::env::current_dir().ok()?;

    loop {
        let candidate = current.join(".makeitso");
        if candidate.exists() && candidate.is_dir() {
            return Some(current);
        }

        if !current.pop() {
            break;
        }
    }

    // If we reach here, we didn't find the project root
    // This might be totally expected depending on the context
    None
}