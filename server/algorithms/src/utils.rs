use std::fmt::Debug;
use std::fs::{create_dir_all, File};
use std::io::{Result, Write};

pub fn debug_hashmap<T, U>(file_name: &str, input: &T) -> Result<()>
where
    U: Debug,
    T: Debug + Clone + IntoIterator<Item = U>,
{
    create_dir_all("./debug_files")?;
    let path = format!("./debug_files/{}", file_name);
    let mut content: String = "".to_string();

    for x in input.clone().into_iter() {
        content = format!("{}\n{:?}\n", content, x);
    }
    content = content.trim_end_matches(",").to_string();
    let mut output = File::create(path)?;
    write!(output, "{}", content)?;
    // println!("Saved {} to file with {} coords", file_name, input.len());
    Ok(())
}
