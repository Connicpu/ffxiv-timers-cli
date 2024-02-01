use std::{
    ffi::OsStr,
    fs::read_to_string,
    io::Write,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Local, Utc};
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

#[cfg(target_os = "windows")]
const SUBTRACKER_FOLDER: &str = r#"AppData\Roaming\XIVLauncher\pluginConfigs\SubmarineTracker"#;
#[cfg(target_os = "linux")]
const SUBTRACKER_FOLDER: &str = ".xlcore/pluginConfigs/SubmarineTracker";

fn main() -> anyhow::Result<()> {
    let user_dirs = directories::UserDirs::new().unwrap();
    let sub_folder: PathBuf = [user_dirs.home_dir(), Path::new(SUBTRACKER_FOLDER)]
        .iter()
        .collect();

    let mut stdout = StandardStream::stdout(termcolor::ColorChoice::Always);
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
        if data.submarines.is_empty() {
            continue;
        }

        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Rgb(255, 255, 255))))?;
        writeln!(
            &mut stdout,
            "Submarines | {char} «{tag}» ({world}) | {count}",
            world = data.world,
            char = data.character_name,
            tag = data.tag,
            count = data.submarines.len()
        )?;
        let max_name_length = data
            .submarines
            .iter()
            .map(|sub| sub.name.len())
            .max()
            .unwrap();
        for sub in data.submarines {
            let name = &*sub.name;
            let now = Utc::now();
            let time = sub.return_time.with_timezone(&Local);
            if sub.return_time == DateTime::<Utc>::default() {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Magenta)))?;
                writeln!(&mut stdout, "    {name:^max_name_length$} - Unassigned")?;
            } else if sub.return_time <= now {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
                writeln!(&mut stdout, "    {name:^max_name_length$} - Voyage complete")?;
            } else {
                let dur = sub.return_time - now;
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)))?;
                let time_fmt = time.format("%Y-%m-%d %H:%M:%S");
                writeln!(
                    &mut stdout,
                    "    {name:<max_name_length$} - {:02}:{:02}:{:02} ({time_fmt})",
                    dur.num_hours(),
                    dur.num_minutes() % 60,
                    dur.num_seconds() % 60
                )?;
            }
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
