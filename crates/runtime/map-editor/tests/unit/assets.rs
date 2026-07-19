use super::*;

#[test]
fn loads_the_real_tile_catalog_and_default_project() {
    let assets = load_assets().unwrap();
    assert!(assets.ids.len() > 200);
    assert!(assets.ids.iter().all(|id| id.as_str() != "tile-0030"));
    assert!(
        assets
            .project_ids
            .iter()
            .any(|id| id.as_str() == "tile-0030")
    );
    assert!(load_project(&default_project_path(), &assets.project_ids).is_ok());
    let project = project_from_json_or_default(None, &assets.ids).unwrap();
    assert_eq!((project.width, project.height), (24, 16));
}
