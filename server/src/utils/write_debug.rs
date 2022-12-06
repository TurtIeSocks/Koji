use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::{create_dir_all, File};
use std::io::{Result, Write};

pub fn hashmap<T, U>(file_name: &str, input: &HashMap<T, U>) -> Result<()>
where
    T: Debug,
    U: Debug,
{
    create_dir_all("./debug_files")?;
    let path = format!("./debug_files/{}", file_name);
    let mut content: String = "".to_string();

    for (key, value) in input.iter() {
        content = format!("{}\n{:?} | {:?}\n", content, key, value);
    }
    content = content.trim_end_matches(",").to_string();
    let mut output = File::create(path)?;
    write!(output, "{}", content)?;
    println!("Saved {} to file with {} coords", file_name, input.len());
    Ok(())
}
