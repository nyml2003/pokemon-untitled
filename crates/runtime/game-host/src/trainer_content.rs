use std::{error::Error, fs, io, path::PathBuf};

use game_foundation::TrainerCatalog;

pub fn load_trainer_catalog() -> Result<TrainerCatalog, Box<dyn Error>> {
    let path = trainer_catalog_path();
    let json = fs::read_to_string(&path)?;
    TrainerCatalog::from_json(&json).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("trainer catalog {}: {error}", path.display()),
        )
        .into()
    })
}

fn trainer_catalog_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../assets/source/trainer/trainers-v1.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_in_trainer_catalog_loads() -> Result<(), Box<dyn Error>> {
        let catalog = load_trainer_catalog()?;
        assert_eq!(catalog.trainers().len(), 1);
        Ok(())
    }
}
