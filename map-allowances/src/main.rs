use std::{
    ffi::OsStr,
    fmt,
    fs::read_to_string,
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
};

use chrono::{DateTime, Duration, Local, SubsecRound, Utc};
use serde::{de::Visitor, Deserialize, Deserializer, Serialize};
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

#[cfg(target_os = "windows")]
const TASKS_FOLDER: &str = r#"AppData\Roaming\XIVLauncher\pluginConfigs\Accountant\tasks"#;
#[cfg(target_os = "linux")]
const TASKS_FOLDER: &str = ".xlcore/pluginConfigs/Accountant/tasks";

#[derive(Serialize, Deserialize)]
struct AccountantTaskData {
    #[serde(rename = "Item1")]
    char_info: CharacterInfo,
    #[serde(rename = "Item2")]
    task_info: TaskInfo,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CharacterInfo {
    name: String,
    server_id: i32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct TaskInfo {
    #[serde(deserialize_with = "datetime_or_default")]
    map: DateTime<Utc>,
}

fn server_name(id: i32) -> &'static str {
    match id {
        72 => "Tonberry",
        91 => "Balmung",
        _ => "(Unknown Server)",
    }
}

fn main() -> anyhow::Result<()> {
    let user_dirs = directories::UserDirs::new().unwrap();
    let tasks_folder = PathBuf::from_iter([user_dirs.home_dir(), Path::new(TASKS_FOLDER)]);

    let mut stdout = StandardStream::stdout(termcolor::ColorChoice::Always);

    let mut entries = Vec::new();
    for entry in tasks_folder.read_dir()? {
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
            stdout.set_color(&error_color())?;
            eprintln!("Failed to open {:?}", path);
            continue;
        };
        let data = match serde_json::from_str::<AccountantTaskData>(&contents) {
            Ok(data) => data,
            Err(err) => {
                stdout.set_color(&error_color())?;
                eprintln!("Failed to deserialize {:?}", path);
                eprintln!("{:#?}", err);
                continue;
            }
        };

        entries.push(data);
    }

    if entries.is_empty() {
        return Ok(());
    }

    entries.retain(|entry| {
        let now = Utc::now();
        let one_week_ago = now - Duration::weeks(1);
        entry.task_info.map > one_week_ago
    });

    let max_name_len = entries
        .iter()
        .map(|entry| entry.char_info.name.len() + server_name(entry.char_info.server_id).len() + 3)
        .max()
        .unwrap();

    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Rgb(255, 255, 255))))?;
    writeln!(&mut stdout, "Map Allowances")?;
    for data in entries {
        let now = Utc::now();

        let time = data.task_info.map.with_timezone(&Local).round_subsecs(0);
        let time_display = if data.task_info.map < now {
            stdout.set_color(&ready_color())?;
            format!("{time}")
        } else {
            stdout.set_color(&waiting_color())?;
            let dur = data.task_info.map - now;
            format!(
                "{:02}:{:02}:{:02}",
                dur.num_hours(),
                dur.num_minutes() % 60,
                dur.num_seconds() % 60
            )
        };

        let char_name = &*data.char_info.name;
        let char_server = server_name(data.char_info.server_id);

        let name_display = format!("{char_name} ({char_server})");
        let time_fmt = time.format("%Y-%m-%d %H:%M:%S");
        writeln!(
            &mut stdout,
            "    {name_display:<max_name_len$} - {time_display} ({time_fmt})"
        )?;
    }

    Ok(())
}

fn error_color() -> ColorSpec {
    ColorSpec::new().set_fg(Some(Color::Red)).clone()
}

fn ready_color() -> ColorSpec {
    ColorSpec::new().set_fg(Some(Color::Green)).clone()
}

fn waiting_color() -> ColorSpec {
    ColorSpec::new().set_fg(Some(Color::Cyan)).clone()
}

fn datetime_or_default<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    struct DateTimeOrDefault;

    impl<'de> Visitor<'de> for DateTimeOrDefault {
        type Value = DateTime<Utc>;

        fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
            fmt.write_str("string")
        }

        fn visit_str<E>(self, value: &str) -> Result<DateTime<Utc>, E>
        where
            E: serde::de::Error,
        {
            Ok(DateTime::from_str(value).unwrap_or_default())
        }
    }

    deserializer.deserialize_str(DateTimeOrDefault)
}
