use configparser::ini::Ini;
use std::{collections::HashMap, sync::Arc};
use sdl2::keyboard::Keycode;
use log::{debug,error,warn};

pub struct Cfg {
    keyboard_layout: Arc<HashMap<Keycode, u8>>,
}

impl Default for Cfg {
    fn default() -> Self {
        Self {
            keyboard_layout: Arc::new(HashMap::<Keycode,u8>::new()),
        }
    }
}

impl Cfg {
    pub fn get_u8_from_keycode(&self, k: &Keycode) -> Option<u8> {
        self.keyboard_layout.get(k).copied()
    }
    /// Load a config file which defines a map of keys on keyboard to CHIP-8 layout
    /// Takes filepath as &String
    pub fn load_config(&mut self, filepath: &str) -> &mut Self {
        let mut config = Ini::new();
        let map = config.load(filepath).unwrap();
        let layout: Arc<HashMap<Keycode, u8>>;
        let heading = "keyboard_layout";
        let parsed_heading = map.get(heading);

        match parsed_heading {
            Some(map) => {
                debug!("Loaded {heading} from config file");
                layout = Arc::new(map
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
                self.keyboard_layout = layout;
            },
            None => {
                error!("Unable to load {heading} from config file");
            }
        }
        self
    }
}
