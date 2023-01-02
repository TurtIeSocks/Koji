use super::*;

pub mod auth;
pub mod response;

pub fn is_docker() -> io::Result<bool> {
    let mut path = env::current_dir()?;
    path.push("dist");
    let metadata = fs::metadata(path)?;
    Ok(metadata.is_dir())
}
