use std::{
    collections::HashMap,
    fs::read_to_string,
    io::{Cursor, Write},
    path::{Path, PathBuf},
};

use serde::Deserialize;
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

#[cfg(target_os = "windows")]
const INVENTORY_FILE: &str =
    r#"AppData\Roaming\XIVLauncher\pluginConfigs\InventoryTools\inventories.csv"#;
#[cfg(target_os = "linux")]
const INVENTORY_FILE: &str = ".xlcore/pluginConfigs/InventoryTools/inventories.csv";

#[cfg(target_os = "windows")]
const INVENTORY_META_FILE: &str =
    r#"AppData\Roaming\XIVLauncher\pluginConfigs\InventoryTools.json"#;
#[cfg(target_os = "linux")]
const INVENTORY_META_FILE: &str = ".xlcore/pluginConfigs/InventoryTools.json";

const HEADER: &str = "container,slot,item_id,quantity,spiritbond,condition,flags,\
                      materia1,materia2,materia3,materia4,materia5,\
                      materia_grade1,materia_grade2,materia_grade3,materia_grade4,materia_grade5,\
                      stain,glamour_id,unk1,unk2,unk3,character_id,unk4,gearset_ids,gearset_names\n";

#[derive(Deserialize)]
struct InventoryItem {
    item_id: u32,
    quantity: u32,
    character_id: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MetaConfig {
    saved_characters: HashMap<String, SavedCharacter>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SavedCharacter {
    name: String,
    world_id: u32,
}

fn worldname(id: u32) -> &'static str {
    match id {
        72 => "Tonberry",
        91 => "Balmung",
        _ => "<Unknown>",
    }
}

fn main() -> anyhow::Result<()> {
    let mut stdout = StandardStream::stdout(termcolor::ColorChoice::Always);
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Rgb(255, 255, 255))))?;

    let user_dirs = directories::UserDirs::new().unwrap();
    let conf_path: PathBuf = [user_dirs.home_dir(), Path::new(INVENTORY_META_FILE)]
        .iter()
        .collect();
    let conf_data = read_to_string(&conf_path)?;
    let conf: MetaConfig = serde_json::from_str(&conf_data)?;

    let inv_path: PathBuf = [user_dirs.home_dir(), Path::new(INVENTORY_FILE)]
        .iter()
        .collect();
    let inv_data = String::from(HEADER) + &read_to_string(&inv_path)?;
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(Cursor::new(inv_data));
    for res in reader.deserialize() {
        let item: InventoryItem = res?;
        if item.item_id == 21072 {
            let savedchar = conf.saved_characters.get(&item.character_id.to_string());
            let charname = savedchar.map(|chr| &*chr.name).unwrap_or("");
            let worldname = savedchar.map(|chr| worldname(chr.world_id)).unwrap_or("");
            writeln!(
                &mut stdout,
                "{charname} ({worldname}) has {} ventures",
                item.quantity
            )?;
        }
    }

    Ok(())
}
