use super::*;

struct MemoryStorage(String);
impl eframe::Storage for MemoryStorage {
    fn get_string(&self, _: &str) -> Option<String> {
        Some(self.0.clone())
    }
    fn set_string(&mut self, _: &str, value: String) {
        self.0 = value;
    }
    fn flush(&mut self) {}
}

fn legacy() -> MemoryStorage {
    MemoryStorage(r#"(enabled:true,min_cps:8.5,max_cps:17.5,cps:13.0,suspend:"Mouse 5",hotkey:"V",avoid_gui:true,humanize:false,jitter:false,only_ingame:true)"#.into())
}

#[test]
fn old_configs_keep_citron_and_existing_controls() {
    let clicker: Clicker = eframe::get_value(&legacy(), "clicker").unwrap();
    assert_eq!(clicker.randomization, Randomization::Citron);
    assert!(!clicker.humanize);
    assert_eq!(
        (clicker.min_cps, clicker.max_cps, clicker.cps),
        (8.5, 17.5, 13.0)
    );
}

#[test]
fn every_mode_persists_and_reaches_both_workers() {
    let mut storage = legacy();
    let mut clicker: Clicker = eframe::get_value(&storage, "clicker").unwrap();
    for mode in Randomization::ALL {
        clicker.randomization = mode;
        eframe::set_value(&mut storage, "clicker", &clicker);
        let restored: Clicker = eframe::get_value(&storage, "clicker").unwrap();
        assert_eq!(restored.randomization, mode);
        for left in [true, false] {
            let snap = snap_of(&restored, left);
            assert_eq!(snap.randomization, mode);
            assert_eq!((snap.min_cps, snap.max_cps), (8.5, 17.5));
            assert_eq!(snap.is_left, left);
        }
    }
}
