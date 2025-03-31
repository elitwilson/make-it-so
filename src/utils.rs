use std::path::PathBuf;
use anyhow::Result;

pub fn find_project_root() -> Result<PathBuf> {
  let mut current = std::env::current_dir()?;

  loop {
      let candidate = current.join(".makeitso");
      if candidate.exists() && candidate.is_dir() {
          return Ok(current);
      }

      if !current.pop() {
          break;
      }
  }

  anyhow::bail!(
      "🛑 Couldn't find a .makeitso/ directory in this or any parent folder.\n\
       → Are you inside a Make It So project?\n\
       → If not, run `mis init` in your project root."
  )
}