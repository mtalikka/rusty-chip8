use configparser::ini::Ini;
use std::{collections::HashMap, env};
use sdl2::keyboard::Keycode;
use log::{debug,error,warn};

const DEFAULT_LAYOUT: [Keycode; 16] = [
    Keycode::X,
    Keycode::NUM_1,
    Keycode::NUM_2,
    Keycode::NUM_3,
    Keycode::Q,
    Keycode::W,
    Keycode::E,
    Keycode::A,
    Keycode::S,
    Keycode::D,
    Keycode::Z,
    Keycode::C,
    Keycode::NUM_4,
    Keycode::R,
    Keycode::F,
    Keycode::V,
];

pub struct Cfg {
    keyboard_layout: HashMap<Keycode, u8>,
}

impl Default for Cfg {
    fn default() -> Self {
        Self {
            keyboard_layout: HashMap::<Keycode,u8>::new(),
        }
    }
}

impl Cfg {
    pub fn get_u8_from_keycode(&self, k: Keycode) -> Option<&u8> {
        if self.keyboard_layout.is_empty() {
            error!("Keyboard layout is empty");
            return None
        }
        self.keyboard_layout.get(&k)
    }
    /// Load a config file which defines a map of keys on keyboard to CHIP-8 layout
    /// Takes filepath as &String
    pub fn load_config(&mut self, filepath: &str) -> &mut Self {
        let mut config = Ini::new();
        let mut path: String = match env::current_dir() {
            Ok(val) => {
                val.display().to_string()
            },
            Err(e) => {
                warn!("Unable to get current directory: [{e}]");
                return self;
            }
        };
        path += "/";
        path += filepath;
        let layout: HashMap<Keycode, u8>;
        // If config file is not found, revert to default keyboard layout
        let raw_map = match config.load(path) {
            Ok(val) => val,
            Err(e) => {
                warn!("Unable to load config file: [{e}]. Using default keyboard lyout.");
                let i: u8 = 0;
                layout = DEFAULT_LAYOUT
                    .iter()
                    .map(
                        |val|
                        {
                            (*val, i)
                        }
                    )
                    .collect::<HashMap<Keycode, u8>>()
                ;
                self.keyboard_layout = layout;
                return self;
            }
        };
        let heading = "keyboard_layout";
        let parsed_heading = raw_map.get(heading);

        match parsed_heading {
            Some(map) => {
                debug!("Loaded heading: {heading} from config file");
                layout = map
                    .iter()
                    .map(
                        |(key, val)| 
                        {
                            let mut k = Keycode::NUM_0;
                            match Keycode::from_name(key) {
                                Some(val) => k = val,
                                None => { warn!("Failed to parse config entry to SDL keycode. Controls may not work as expected.") ; }
                            };
                            let v = val.as_ref().unwrap_or(&u8::MAX.to_string()).parse::<u8>().unwrap();
                            debug!("Mapping {k} with value: {v}");
                            (k, v)
                        }
                    )
                    .collect::<HashMap<Keycode, u8>>();
                // Validate the keys
                for (_, val) in layout.iter() {
                    if *val == u8::MAX {
                        warn!("Unable to extract key value from config file.")
                    }
                }
                self.keyboard_layout = layout.clone();
            },
            None => {
                error!("Unable to load {heading} from config file");
            }
        }
        self
    }
}
