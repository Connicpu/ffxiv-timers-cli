use std::{
    ffi::OsStr,
    fs::read_to_string,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Local, Utc};

#[cfg(target_os = "windows")]
const SUBTRACKER_FOLDER: &str = r#"AppData\Roaming\XIVLauncher\pluginConfigs\SubmarineTracker"#;
#[cfg(target_os = "linux")]
const SUBTRACKER_FOLDER: &str = ".xlcore/pluginConfigs/SubmarineTracker";

fn main() -> anyhow::Result<()> {
    let user_dirs = directories::UserDirs::new().unwrap();
    let sub_folder: PathBuf = [user_dirs.home_dir(), Path::new(SUBTRACKER_FOLDER)]
        .iter()
        .collect();

    for entry in sub_folder.read_dir()? {
        let Ok(entry) = entry else { continue };
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if !kind.is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension() != Some(OsStr::new("json")) {
            continue;
        }
        let Ok(contents) = read_to_string(&path) else {
            eprintln!("Failed to open {:?}", path);
            continue;
        };
        let Ok(data) = serde_json::from_str::<Character>(&contents) else {
            eprintln!("Failed to deserialize {:?}", path);
            continue;
        };

        println!(
            "{world}: {char} «{tag}» | {count}",
            world = data.world,
            char = data.character_name,
            tag = data.tag,
            count = data.submarines.len()
        );
        for sub in data.submarines {
            let time = sub.return_time.with_timezone(&Local);
            let status = match sub.return_time {
                time if time == DateTime::<Utc>::default() => "🏡",
                time if time < Utc::now() => "✅",
                _ => "🕛",
            };
            println!("    {name} - Returns {time} {status}", name = sub.name);
        }
    }

    Ok(())
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Character {
    pub character_name: String,
    pub world: String,
    pub tag: String,
    pub submarines: Vec<Submarine>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Submarine {
    pub name: String,
    pub return_time: DateTime<Utc>,
}
