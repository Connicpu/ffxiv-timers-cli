use std::{
    io::Write,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Local, TimeZone, Utc};
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

#[cfg(target_os = "windows")]
const SUBTRACKER_FOLDER: &str = r#"AppData\Roaming\XIVLauncher\pluginConfigs\SubmarineTracker"#;
#[cfg(target_os = "linux")]
const SUBTRACKER_FOLDER: &str = ".xlcore/pluginConfigs/SubmarineTracker";

fn main() -> anyhow::Result<()> {
    let db = open_db()?;
    let fcs = get_submarine_info(&db)?;

    let mut stdout = StandardStream::stdout(termcolor::ColorChoice::Always);
    for fc in fcs {

        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Rgb(255, 255, 255))))?;
        writeln!(
            &mut stdout,
            "Submarines | {char} «{tag}» ({world}) | {count}",
            world = fc.world,
            char = fc.character_name,
            tag = fc.tag,
            count = fc.submarines.len()
        )?;
        let max_name_length = fc
            .submarines
            .iter()
            .map(|sub| sub.name.len())
            .max()
            .unwrap();
        for sub in fc.submarines {
            let name = &*sub.name;
            let now = Utc::now();
            let time = sub.return_time.with_timezone(&Local);
            if sub.return_time == DateTime::<Utc>::default() {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Magenta)))?;
                writeln!(&mut stdout, "    {name:^max_name_length$} - Unassigned")?;
            } else if sub.return_time <= now {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
                writeln!(
                    &mut stdout,
                    "    {name:^max_name_length$} - Voyage complete"
                )?;
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

fn open_db() -> anyhow::Result<rusqlite::Connection> {
    let user_dirs = directories::UserDirs::new().unwrap();
    let sub_db_file: PathBuf = [
        user_dirs.home_dir(),
        Path::new(SUBTRACKER_FOLDER),
        Path::new("submarine-sqlite.db"),
    ]
    .iter()
    .collect();
    let db = rusqlite::Connection::open_with_flags(
        sub_db_file,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    Ok(db)
}

fn get_submarine_info(db: &rusqlite::Connection) -> anyhow::Result<Vec<FreeCompany>> {
    const QUERY: &str = "
        SELECT
            freecompany.FreeCompanyId as fc_id,
            freecompany.CharacterName as character_name,
            freecompany.World as world,
            freecompany.FreeCompanyTag as tag,
            submarine.SubmarineId as sub_id,
            submarine.Name AS sub_name, 
            submarine.Return AS return_time
        FROM submarine JOIN freecompany ON submarine.FreeCompanyId = freecompany.FreeCompanyId
        ORDER BY world, tag, fc_id, sub_id
    ";

    let mut stmt = db.prepare(QUERY)?;
    let mut fcs: Vec<FreeCompany> = vec![];
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let fc_id: Vec<u8> = row.get("fc_id")?;
        if fcs.is_empty() || fcs.last().unwrap().id != fc_id {
            fcs.push(FreeCompany {
                id: fc_id,
                character_name: row.get("character_name")?,
                world: row.get("world")?,
                tag: row.get("tag")?,
                submarines: vec![],
            });
        }

        let fc = fcs.last_mut().unwrap();
        let timestamp = row.get("return_time")?;
        fc.submarines.push(Submarine {
            name: row.get("sub_name")?,
            return_time: Utc.timestamp_opt(timestamp, 0).single().unwrap(),
        });
    }
    Ok(fcs)
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct FreeCompany {
    pub id: Vec<u8>,
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
