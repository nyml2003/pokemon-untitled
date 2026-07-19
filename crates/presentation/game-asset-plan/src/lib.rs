//! Pure planning and assembly of the game's native assets.

#![forbid(unsafe_code)]

use std::{collections::BTreeSet, error::Error, fmt};

use battle_application::{MoveCategory, PokemonType, TEAM_SIZE};
use game_assets::{AssetKey, DecodedImage, decode_png};
use game_data::PokedexData;
use game_native_plan::NativeAssets;
use game_session::DemoSpriteManifest;
use game_ui::WorldAnimation;
use game_view::{
    move_category_icon_asset, opponent_front_asset, pill_ui_asset, player_back_asset,
    pokemon_icon_asset, rounded_ui_asset, type_icon_asset, world_character_asset,
};
use punctum_gpu::{PixelSize, Rgba8};
use world_application::{CharacterAppearanceId, Direction, WorldObservation};

const PLAYER_CHARACTER_ASSETS: [[&str; 6]; 4] = [
    [
        "character/red/down/stand/00",
        "character/red/down/walk/02",
        "character/red/down/walk/03",
        "character/red/down/run/01",
        "character/red/down/run/02",
        "character/red/down/run/03",
    ],
    [
        "character/red/left/stand/00",
        "character/red/left/walk/01",
        "character/red/left/walk/02",
        "character/red/left/run/01",
        "character/red/left/run/02",
        "character/red/left/run/03",
    ],
    [
        "character/red/right/stand/00",
        "character/red/right/walk/01",
        "character/red/right/walk/02",
        "character/red/right/run/01",
        "character/red/right/run/02",
        "character/red/right/run/03",
    ],
    [
        "character/red/up/stand/00",
        "character/red/up/walk/01",
        "character/red/up/walk/02",
        "character/red/up/run/01",
        "character/red/up/run/02",
        "character/red/up/run/03",
    ],
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetRequest {
    pub resource_key: AssetKey,
    pub asset_key: AssetKey,
    pub expected_size: Option<PixelSize>,
}

pub struct AssetBytes {
    pub request: AssetRequest,
    pub bytes: Vec<u8>,
}

pub fn asset_requests(
    manifest: &DemoSpriteManifest,
    pokedex: &PokedexData,
    world: &WorldObservation,
) -> Vec<AssetRequest> {
    debug_assert_eq!(manifest.player().len(), TEAM_SIZE);
    debug_assert_eq!(manifest.opponent().len(), TEAM_SIZE);
    let mut requests = character_requests(world);
    for (slot, form) in manifest.player().iter().enumerate() {
        for frame in 0..2 {
            requests.push(AssetRequest {
                resource_key: player_back_asset(slot, frame),
                asset_key: pokemon_asset_key(form.0, "normal", "back", frame),
                expected_size: None,
            });
            requests.push(AssetRequest {
                resource_key: pokemon_icon_asset(slot, frame),
                asset_key: pokemon_asset_key(form.0, "normal", "front", frame),
                expected_size: None,
            });
        }
    }
    for (slot, form) in manifest.opponent().iter().enumerate() {
        for frame in 0..2 {
            requests.push(AssetRequest {
                resource_key: opponent_front_asset(slot, frame),
                asset_key: pokemon_asset_key(form.0, "normal", "front", frame),
                expected_size: None,
            });
        }
    }
    requests.extend(type_icon_requests());
    requests.extend(move_category_requests());
    for entry in pokedex.entries() {
        requests.push(AssetRequest {
            resource_key: AssetKey::new(format!("pokedex/{}", entry.national_dex))
                .expect("fixed Pokedex resource key is valid"),
            asset_key: AssetKey::new(entry.front_asset.clone())
                .expect("generated Pokedex asset key is valid"),
            expected_size: None,
        });
    }
    requests
}

fn character_requests(world: &WorldObservation) -> Vec<AssetRequest> {
    let appearances = world
        .actors()
        .iter()
        .map(|actor| actor.appearance().clone())
        .collect::<BTreeSet<_>>();
    let mut requests = Vec::with_capacity(appearances.len() * 24);
    for appearance in appearances {
        if appearance.as_str() == "red" {
            requests.extend(player_character_requests(&appearance));
        } else {
            requests.extend(basic_character_requests(&appearance));
        }
    }
    requests
}

fn player_character_requests(appearance: &CharacterAppearanceId) -> Vec<AssetRequest> {
    let directions = [
        Direction::Down,
        Direction::Left,
        Direction::Right,
        Direction::Up,
    ];
    let mut requests = Vec::with_capacity(24);
    for (direction_index, assets) in PLAYER_CHARACTER_ASSETS.iter().enumerate() {
        for (frame, asset) in assets.iter().enumerate() {
            requests.push(AssetRequest {
                resource_key: world_character_asset(
                    appearance,
                    directions[direction_index],
                    frame_animation(frame),
                    frame_index(frame),
                ),
                asset_key: AssetKey::new(*asset).expect("fixed character key is valid"),
                expected_size: None,
            });
        }
    }
    requests
}

fn basic_character_requests(appearance: &CharacterAppearanceId) -> Vec<AssetRequest> {
    let directions = [
        Direction::Down,
        Direction::Left,
        Direction::Right,
        Direction::Up,
    ];
    let mut requests = Vec::with_capacity(12);
    for direction in directions {
        for (animation, sprite_frame, action, action_frame) in [
            (WorldAnimation::Stand, 0, "stand", 0),
            (WorldAnimation::Walk, 0, "walk", 1),
            (WorldAnimation::Walk, 2, "walk", 2),
        ] {
            requests.push(AssetRequest {
                resource_key: world_character_asset(appearance, direction, animation, sprite_frame),
                asset_key: AssetKey::new(format!(
                    "character/{}/{}/{action}/{action_frame:02}",
                    appearance.as_str(),
                    direction_name(direction),
                ))
                .expect("generated character asset key is valid"),
                expected_size: None,
            });
        }
    }
    requests
}

const fn direction_name(direction: Direction) -> &'static str {
    match direction {
        Direction::Down => "down",
        Direction::Left => "left",
        Direction::Right => "right",
        Direction::Up => "up",
    }
}

fn frame_animation(frame: usize) -> WorldAnimation {
    match frame {
        0 => WorldAnimation::Stand,
        1 | 2 => WorldAnimation::Walk,
        _ => WorldAnimation::Run,
    }
}

const fn frame_index(frame: usize) -> usize {
    match frame {
        0 => 0,
        1 => 0,
        2 => 2,
        3 => 1,
        4 => 0,
        _ => 2,
    }
}

fn type_icon_requests() -> Vec<AssetRequest> {
    let types = [
        PokemonType::Normal,
        PokemonType::Fighting,
        PokemonType::Flying,
        PokemonType::Poison,
        PokemonType::Ground,
        PokemonType::Rock,
        PokemonType::Bug,
        PokemonType::Ghost,
        PokemonType::Steel,
        PokemonType::Fire,
        PokemonType::Water,
        PokemonType::Grass,
        PokemonType::Electric,
        PokemonType::Psychic,
        PokemonType::Ice,
        PokemonType::Dragon,
        PokemonType::Dark,
    ];
    types
        .into_iter()
        .map(|pokemon_type| {
            let key = type_icon_asset(pokemon_type);
            AssetRequest {
                resource_key: key.clone(),
                asset_key: key,
                expected_size: None,
            }
        })
        .collect()
}

fn move_category_requests() -> Vec<AssetRequest> {
    [
        (MoveCategory::Physical, "physical.png"),
        (MoveCategory::Special, "special.png"),
        (MoveCategory::Status, "status.png"),
    ]
    .into_iter()
    .map(|(category, _)| {
        let key = move_category_icon_asset(category);
        AssetRequest {
            resource_key: key.clone(),
            asset_key: key,
            expected_size: None,
        }
    })
    .collect()
}

fn pokemon_asset_key(form: u32, palette: &str, pose: &str, frame: usize) -> AssetKey {
    AssetKey::new(format!(
        "pokemon/{form:04}/form/00/{palette}/{pose}/{:02}",
        frame % 2
    ))
    .expect("fixed pokemon key is valid")
}

pub fn assemble_assets(
    sources: Vec<AssetBytes>,
    map_images: Vec<(AssetKey, DecodedImage)>,
) -> Result<NativeAssets, GameAssetError> {
    let mut images = vec![(
        AssetKey::new("solid/white").expect("the white asset key is valid"),
        DecodedImage::solid(Rgba8::new(255, 255, 255, 255)),
    )];
    images.push((rounded_ui_asset(), rounded_mask(64, 64, 6)));
    images.push((pill_ui_asset(), rounded_mask(128, 64, 32)));
    for source in sources {
        let image = decode_png(&source.bytes).map_err(|error| GameAssetError::Decode {
            path: source.request.asset_key.as_str().into(),
            message: error.to_string(),
        })?;
        if source
            .request
            .expected_size
            .is_some_and(|expected| image.size() != expected)
        {
            return Err(GameAssetError::WrongSize {
                path: source.request.asset_key.as_str().into(),
                expected: source.request.expected_size.expect("the size was checked"),
                actual: image.size(),
            });
        }
        images.push((source.request.resource_key, image));
    }
    images.extend(map_images);
    NativeAssets::new(images).map_err(|error| GameAssetError::Assets(error.to_string()))
}

fn rounded_mask(width: u32, height: u32, radius: u32) -> DecodedImage {
    let mut rgba8 = Vec::with_capacity((width * height * 4) as usize);
    let radius = radius as f32;
    let half_width = width as f32 / 2.0;
    let half_height = height as f32 / 2.0;
    let inner_x = half_width - radius;
    let inner_y = half_height - radius;
    for y in 0..height {
        for x in 0..width {
            let dx = ((x as f32 + 0.5) - half_width).abs() - inner_x;
            let dy = ((y as f32 + 0.5) - half_height).abs() - inner_y;
            let outside = dx.max(0.0).hypot(dy.max(0.0));
            let inside = dx.max(dy).min(0.0);
            let distance = outside + inside - radius;
            let alpha = ((0.5 - distance).clamp(0.0, 1.0) * 255.0).round() as u8;
            rgba8.extend_from_slice(&[255, 255, 255, alpha]);
        }
    }
    DecodedImage::from_rgba8(PixelSize::new(width, height), rgba8)
        .expect("generated UI masks have a complete RGBA8 payload")
}

#[derive(Debug)]
pub enum GameAssetError {
    Decode {
        path: String,
        message: String,
    },
    WrongSize {
        path: String,
        expected: PixelSize,
        actual: PixelSize,
    },
    Assets(String),
}

impl fmt::Display for GameAssetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode { path, message } => {
                write!(formatter, "failed to decode sprite {path}: {message}")
            }
            Self::WrongSize {
                path,
                expected,
                actual,
            } => write!(
                formatter,
                "sprite {path} must be {}x{} pixels, received {}x{}",
                expected.width, expected.height, actual.width, actual.height
            ),
            Self::Assets(message) => write!(formatter, "failed to build game assets: {message}"),
        }
    }
}

impl Error for GameAssetError {}

#[cfg(test)]
mod tests {
    use game_data::CurrentDataSet;
    use game_session::GameSession;
    use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};
    use map_project::{
        AtomicTileId, CharacterAppearanceId, CompositeTile, CompositeTileId, MapActor, MapActorId,
        MapDirection, MapProject, MapProjectId, TilePosition,
    };
    use world_application::WorldApplication;

    use super::*;

    fn manifest() -> DemoSpriteManifest {
        GameSession::new_demo(CurrentDataSet::embedded().unwrap(), 17)
            .unwrap()
            .sprite_manifest()
            .unwrap()
    }

    fn world() -> world_application::WorldObservation {
        GameSession::new_demo(CurrentDataSet::embedded().unwrap(), 17)
            .unwrap()
            .snapshot()
            .world()
            .clone()
    }

    fn world_with_npc() -> world_application::WorldObservation {
        let tile = AtomicTileId::new("tile-0001").unwrap();
        let material = CompositeTileId::new("ground").unwrap();
        let mut project = MapProject::blank(
            MapProjectId::new("npc-test").unwrap(),
            3,
            1,
            Some(CompositeTile::new(material, vec![tile])),
        );
        project.player_spawn = TilePosition::new(0, 0);
        project.actors.push(MapActor::new(
            MapActorId::new("guide").unwrap(),
            TilePosition::new(1, 0),
            MapDirection::Left,
            CharacterAppearanceId::new("dppt/000").unwrap(),
        ));
        WorldApplication::from_map_project(&project)
            .unwrap()
            .observe()
            .unwrap()
    }

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        PngEncoder::new(&mut bytes)
            .write_image(
                &vec![255; (width * height * 4) as usize],
                width,
                height,
                ExtendedColorType::Rgba8,
            )
            .unwrap();
        bytes
    }

    #[test]
    fn manifest_expands_to_stable_resource_and_asset_key_requests() {
        let pokedex = PokedexData::embedded_gen3().unwrap();
        let requests = asset_requests(&manifest(), &pokedex, &world());
        assert_eq!(requests.len(), 466);
        assert_eq!(
            requests[0].asset_key.as_str(),
            "character/red/down/stand/00"
        );
        assert_eq!(requests[0].resource_key.as_str(), "character/red/0/0");
        assert!(requests.iter().any(|request| {
            request.resource_key == player_back_asset(0, 0)
                && request.asset_key.as_str().contains("/normal/back/00")
        }));
        assert!(requests.iter().any(|request| {
            request.resource_key == pokemon_icon_asset(0, 0)
                && request.asset_key.as_str().contains("/normal/front/00")
        }));
        assert!(requests.iter().any(|request| {
            request.asset_key.as_str() == "ui/battle/move-category/status"
                && request.resource_key.as_str() == "ui/battle/move-category/status"
        }));
    }

    #[test]
    fn npc_appearances_request_only_stand_and_walk_frames() {
        let requests = asset_requests(
            &manifest(),
            &PokedexData::embedded_gen3().unwrap(),
            &world_with_npc(),
        );
        let npc_requests = requests
            .iter()
            .filter(|request| {
                request
                    .asset_key
                    .as_str()
                    .starts_with("character/dppt/000/")
            })
            .collect::<Vec<_>>();
        assert_eq!(npc_requests.len(), 12);
        assert!(
            npc_requests
                .iter()
                .all(|request| !request.asset_key.as_str().contains("/run/"))
        );
        assert!(npc_requests.iter().any(|request| {
            request.resource_key.as_str() == "character/dppt/000/1/2"
                && request.asset_key.as_str() == "character/dppt/000/left/walk/02"
        }));
    }

    #[test]
    fn assembly_decodes_and_validates_declared_sizes() {
        let requests = asset_requests(
            &manifest(),
            &PokedexData::embedded_gen3().unwrap(),
            &world(),
        );
        let sources = requests
            .into_iter()
            .map(|request| AssetBytes {
                bytes: png(
                    request.expected_size.map_or(1, |size| size.width),
                    request.expected_size.map_or(1, |size| size.height),
                ),
                request,
            })
            .collect();
        let assets = assemble_assets(sources, Vec::new()).unwrap();
        assert!(assets.atlas_size().width > 0);
        assert!(assets.resource(&rounded_ui_asset()).is_some());
        assert!(assets.resource(&pill_ui_asset()).is_some());

        let errors = [
            assemble_assets(
                vec![AssetBytes {
                    request: AssetRequest {
                        resource_key: AssetKey::new("bad/png").unwrap(),
                        asset_key: AssetKey::new("bad/png").unwrap(),
                        expected_size: None,
                    },
                    bytes: Vec::new(),
                }],
                Vec::new(),
            )
            .err()
            .unwrap(),
            assemble_assets(
                vec![AssetBytes {
                    request: AssetRequest {
                        resource_key: AssetKey::new("bad/size").unwrap(),
                        asset_key: AssetKey::new("bad/size").unwrap(),
                        expected_size: Some(PixelSize::new(32, 32)),
                    },
                    bytes: png(1, 1),
                }],
                Vec::new(),
            )
            .err()
            .unwrap(),
            assemble_assets(
                vec![AssetBytes {
                    request: AssetRequest {
                        resource_key: AssetKey::new("solid/white").unwrap(),
                        asset_key: AssetKey::new("solid/white").unwrap(),
                        expected_size: None,
                    },
                    bytes: png(1, 1),
                }],
                Vec::new(),
            )
            .err()
            .unwrap(),
        ];
        for error in errors {
            assert!(!error.to_string().is_empty());
        }
    }

    #[test]
    fn generated_ui_masks_have_transparent_corners_and_opaque_centers() {
        for mask in [rounded_mask(64, 64, 6), rounded_mask(128, 64, 32)] {
            let center = ((mask.size().height / 2 * mask.size().width + mask.size().width / 2) * 4
                + 3) as usize;
            assert_eq!(mask.rgba8()[3], 0);
            assert_eq!(mask.rgba8()[center], 255);
        }
    }
}
