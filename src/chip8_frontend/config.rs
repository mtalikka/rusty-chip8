// Load a config file which defines a map of keys on keyboard to CHIP-8 layout
use configparser::ini::Ini;
use std::{collections::HashMap, rc::Rc};
use sdl2::keyboard::Keycode;
use log::{debug,error,warn};

const CFG_FILE_PATH: &str = "../cfg/config.ini";

pub struct Cfg {
    config_file: Ini,
    keyboard_layout: Rc<HashMap<Keycode, u8>>,
}

impl Cfg {
    pub fn get_u8_from_keycode(&self, k: &Keycode) -> Option<u8> {
        self.keyboard_layout.get(k).copied()
    }
}

impl Default for Cfg {
    fn default() -> Self {
        let mut config = Ini::new();
        let map = config.load(CFG_FILE_PATH).unwrap();
        let heading = "keyboard_layout";
        let parsed_heading = map.get(heading);
        let layout: Rc<HashMap<Keycode, u8>>;

        match parsed_heading {
            Some(map) => {
                debug!("Loaded {heading} from config file");
                layout = Rc::new(map
                    .iter()
                    .map(
                        |(key, val)| 
                        {
                            let mut k = Keycode::NUM_0;
                            match Keycode::from_name(key) {
                                Some(val) => k = val,
                                None => { warn!("Failed to parse config entry to SDL keycode. Controls may not work as expected.") ; }
                            };
                            (k, val.as_ref().unwrap_or(&u8::MAX.to_string()).parse::<u8>().unwrap())
                        }
                    )
                    .collect::<HashMap<Keycode, u8>>()
                );
                // Validate the keys
                for (_, val) in layout.as_ref().iter() {
                    if *val == u8::MAX {
                        warn!("Unable to extract key value from config file.")
                    }
                }
            },
            None => {
                error!("Unable to load {heading} from config file");
                layout = Rc::new(HashMap::<Keycode,u8>::new());
            }
        }
        Self {
            config_file: config,
            keyboard_layout: layout,
        }
    }
}