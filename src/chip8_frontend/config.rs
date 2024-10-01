// Load a config file which defines a map of keys on keyboard to CHIP-8 layout
use configparser::ini::Ini;
use std::{collections::HashMap, rc::Rc};

const CFG_FILE_PATH: &str = "../cfg/config.ini";

pub struct Cfg {
    config_file: Ini,
    keyboard_layout: Option<Rc<HashMap<char, u8>>>,
}

impl Default for Cfg {
    fn default() -> Self {
        let mut config = Ini::new();
        let map = config.load(CFG_FILE_PATH).unwrap();
        let heading = "keyboard_layout";
        let parsed_heading = map.get(heading);
        let layout: Option<Rc<HashMap<char, u8>>>;

        match parsed_heading {
            Some(map) => {
                println!("Loaded {heading} from config file");
                layout = Some(Rc::new(map
                    .into_iter()
                    .map(
                        |(key, val)| (
                            key.chars().collect::<Vec<char>>()[0], val.as_ref().unwrap_or(&u8::MAX.to_string()).parse::<u8>().unwrap()
                        )
                    )
                    .collect::<HashMap<char, u8>>()
                ));
                // Validate the keys
                for (_, val) in layout.as_ref().unwrap().iter() {
                    if *val == u8::MAX {
                        println!("Warning: incorrect key detected in config file")
                    }
                }
            },
            None => {
                println!("Unable to load {heading} from config file");
                layout = None.into();
            }
        }
        Self {
            config_file: config,
            keyboard_layout: layout,
        }
    }
}