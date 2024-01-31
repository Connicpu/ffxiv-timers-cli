use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fmt,
    fs::read_to_string,
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
};

use chrono::{DateTime, Duration, Utc};
use serde::{de::Visitor, Deserialize, Deserializer, Serialize};
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

#[cfg(target_os = "windows")]
const CROPDATA_FOLDER: &str = r#"AppData\Roaming\XIVLauncher\pluginConfigs\Accountant\crops_plot"#;
#[cfg(target_os = "linux")]
const CROPDATA_FOLDER: &str = ".xlcore/pluginConfigs/Accountant/crops_plot";

fn crop_name(id: u32) -> &'static str {
    match id {
        4842 => "Almond",
        6146 => "Mirror Apple",
        7604 => "Royal Kukuru",
        7895 => "Sylkis Bud",
        8165 => "Krakka Root",
        12896 => "Old World Fig",
        _ => "(Unknown Crop)",
    }
}

fn crop_grow_time(id: u32) -> Duration {
    match id {
        4842 => Duration::days(5),
        6146 => Duration::days(5),
        7604 => Duration::days(6),
        7895 => Duration::days(5),
        8165 => Duration::days(3),
        12896 => Duration::days(5),
        _ => Duration::zero(),
    }
}

fn crop_wilt_time(id: u32) -> Duration {
    match id {
        4842 => Duration::hours(48),
        6146 => Duration::hours(48),
        7604 => Duration::hours(36),
        7895 => Duration::hours(48),
        8165 => Duration::hours(24),
        12896 => Duration::hours(48),
        _ => Duration::zero(),
    }
}

fn crop_wither_time(id: u32) -> Duration {
    crop_wilt_time(id) + Duration::days(1)
}

#[derive(Serialize, Deserialize)]
struct AccountantCropData {
    #[serde(rename = "Item1")]
    house_info: HouseInfo,
    #[serde(rename = "Item2")]
    crops: Vec<CropInfo>,
}

#[derive(Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct HouseInfo {
    zone: u32,
    server_id: u32,
    ward: u32,
    plot: u32,
}

#[derive(Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CropInfo {
    #[serde(deserialize_with = "datetime_or_default")]
    plant_time: DateTime<Utc>,
    #[serde(deserialize_with = "datetime_or_default")]
    last_tending: DateTime<Utc>,
    plant_id: u32,
    accurate_plant_time: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum CropStatus {
    Good,
    Okay,
    Wilt,
    Done,
    Dead,
}

impl CropStatus {
    fn color(self) -> ColorSpec {
        match self {
            CropStatus::Good => ColorSpec::new().set_fg(Some(Color::Cyan)).clone(),
            CropStatus::Okay => ColorSpec::new().set_fg(Some(Color::Yellow)).clone(),
            CropStatus::Wilt => ColorSpec::new().set_fg(Some(Color::Magenta)).clone(),
            CropStatus::Done => ColorSpec::new().set_fg(Some(Color::Green)).clone(),
            CropStatus::Dead => ColorSpec::new().set_fg(Some(Color::Red)).clone(),
        }
    }
}

fn crop_status(crop: &CropInfo) -> CropStatus {
    let now = Utc::now();
    let wilt_time = crop.last_tending + crop_wilt_time(crop.plant_id);
    let wither_time = crop.last_tending + crop_wither_time(crop.plant_id);
    let finish_time = crop.plant_time + crop_grow_time(crop.plant_id);
    if wither_time < finish_time && wither_time < now {
        CropStatus::Dead
    } else if finish_time < now {
        CropStatus::Done
    } else if finish_time < wither_time {
        CropStatus::Good
    } else if wilt_time < now {
        CropStatus::Wilt
    } else {
        CropStatus::Okay
    }
}

fn main() -> anyhow::Result<()> {
    let user_dirs = directories::UserDirs::new().unwrap();
    let crop_folder: PathBuf = [user_dirs.home_dir(), Path::new(CROPDATA_FOLDER)]
        .iter()
        .collect();

    let mut entries_by_crop: BTreeMap<u32, Vec<(HouseInfo, CropInfo)>> = BTreeMap::new();
    for entry in crop_folder.read_dir()? {
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
        let data = match serde_json::from_str::<AccountantCropData>(&contents) {
            Ok(data) => data,
            Err(err) => {
                eprintln!("Failed to deserialize {:?}", path);
                eprintln!("{:#?}", err);
                continue;
            }
        };

        for crop in data.crops {
            if crop.plant_id == 0 {
                continue;
            }

            entries_by_crop
                .entry(crop.plant_id)
                .or_default()
                .push((data.house_info, crop));
        }
    }

    let mut stdout = StandardStream::stdout(termcolor::ColorChoice::Always);
    stdout.set_color(&ColorSpec::new().set_fg(Some(Color::Rgb(255, 255, 255))))?;
    writeln!(&mut stdout, "Crops:")?;
    for (crop_id, patches) in entries_by_crop {
        let overall_status = patches
            .iter()
            .map(|(_, crop)| crop_status(crop))
            .max()
            .unwrap_or(CropStatus::Okay);

        let stage_time = match overall_status {
            CropStatus::Dead => None,
            CropStatus::Done => None,
            CropStatus::Okay => patches
                .iter()
                .map(|(_, crop)| crop.last_tending + crop_wilt_time(crop.plant_id))
                .min(),
            CropStatus::Wilt => patches
                .iter()
                .map(|(_, crop)| crop.last_tending + crop_wither_time(crop.plant_id))
                .min(),
            CropStatus::Good => patches
                .iter()
                .map(|(_, crop)| crop.plant_time + crop_grow_time(crop.plant_id))
                .min(),
        };

        let now = Utc::now();
        let time_display = stage_time
            .map(|time| time - now)
            .map(|dur| {
                format!(
                    "{:02}:{:02}:{:02}",
                    dur.num_hours(),
                    dur.num_minutes() % 60,
                    dur.num_seconds() % 60
                )
            })
            .unwrap_or_default();

        stdout.set_color(&overall_status.color())?;
        writeln!(
            &mut stdout,
            "   {} ({}) {}",
            crop_name(crop_id),
            patches.len(),
            time_display
        )?;
    }

    Ok(())
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
