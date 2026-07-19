//! Versioned binary storage for validated map projects.

#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
    io::{Read, Write},
};

use map_project::{
    AtomicTileId, CharacterAppearanceId, Collision, CompositeTile, CompositeTileId, MapActor,
    MapActorId, MapDirection, MapError, MapEventKind, MapProject, MapProjectId, TilePixelSize,
    TilePosition, VisualCell,
};

pub const FILE_EXTENSION: &str = "g3mp";
pub const MAGIC: [u8; 4] = *b"G3MP";
pub const CONTAINER_VERSION: u16 = 1;
pub const HEADER_LENGTH: usize = 64;
pub const MAX_MANIFEST_BYTES: usize = 64 * 1024;
pub const MAX_COMPRESSED_PAYLOAD_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_RAW_PAYLOAD_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_DIMENSION: u16 = 4096;
pub const MAX_CELL_COUNT: usize = 4_194_304;

const COMPRESSION_ZSTD: u8 = 1;
const MANIFEST_VERSION: u16 = 1;
const PAYLOAD_SCHEMA_VERSION: u16 = 1;
const SECTION_STRINGS: u8 = 1;
const SECTION_MATERIALS: u8 = 2;
const SECTION_VISUAL: u8 = 3;
const SECTION_COLLISION: u8 = 4;
const SECTION_EVENTS: u8 = 5;
const SECTION_ENTITIES: u8 = 6;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapProjectMetadata {
    pub container_version: u16,
    pub document_format: String,
    pub map_id: String,
    pub tile_size: TilePixelSize,
    pub width: u16,
    pub height: u16,
    pub cell_count: u32,
    pub material_count: u32,
    pub atomic_tile_count: u32,
    pub actor_count: u32,
    pub event_count: u32,
    pub compression: Compression,
    pub compressed_payload_bytes: u64,
    pub raw_payload_bytes: u64,
    pub payload_checksum: [u8; 32],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compression {
    Zstd,
}

impl Compression {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Zstd => "zstd",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WriteOptions {
    pub compression_level: i32,
}

impl Default for WriteOptions {
    fn default() -> Self {
        Self {
            compression_level: 3,
        }
    }
}

#[derive(Debug)]
pub enum MapStorageError {
    Truncated,
    BadMagic,
    UnsupportedContainerVersion(u16),
    InvalidHeader(&'static str),
    InvalidManifest(&'static str),
    LimitExceeded(&'static str),
    DecompressionFailed(String),
    ChecksumMismatch,
    InvalidPayload(&'static str),
    Io(String),
    Map(MapError),
}

impl fmt::Display for MapStorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => formatter.write_str("map storage is truncated"),
            Self::BadMagic => formatter.write_str("map storage has an invalid magic header"),
            Self::UnsupportedContainerVersion(version) => {
                write!(
                    formatter,
                    "unsupported map storage container version {version}"
                )
            }
            Self::InvalidHeader(message) => {
                write!(formatter, "invalid map storage header: {message}")
            }
            Self::InvalidManifest(message) => {
                write!(formatter, "invalid map storage manifest: {message}")
            }
            Self::LimitExceeded(message) => {
                write!(formatter, "map storage exceeds limit: {message}")
            }
            Self::DecompressionFailed(message) => {
                write!(
                    formatter,
                    "cannot decompress map storage payload: {message}"
                )
            }
            Self::ChecksumMismatch => {
                formatter.write_str("map storage payload checksum does not match")
            }
            Self::InvalidPayload(message) => {
                write!(formatter, "invalid map storage payload: {message}")
            }
            Self::Io(message) => write!(formatter, "map storage I/O failed: {message}"),
            Self::Map(error) => write!(formatter, "invalid map project: {error}"),
        }
    }
}

impl Error for MapStorageError {}

impl From<MapError> for MapStorageError {
    fn from(error: MapError) -> Self {
        Self::Map(error)
    }
}

pub struct MapProjectReader;

impl MapProjectReader {
    pub fn inspect(input: &[u8]) -> Result<MapProjectMetadata, MapStorageError> {
        let container = parse_container(input)?;
        parse_manifest(container.manifest, &container.header)
    }

    pub fn read(
        input: &[u8],
        known_tiles: &BTreeSet<AtomicTileId>,
    ) -> Result<MapProject, MapStorageError> {
        let container = parse_container(input)?;
        let metadata = parse_manifest(container.manifest, &container.header)?;
        let raw = decompress(container.payload, &container.header)?;
        decode_project(&raw, &metadata, known_tiles)
    }

    pub fn read_from<R: Read>(
        input: R,
        known_tiles: &BTreeSet<AtomicTileId>,
    ) -> Result<MapProject, MapStorageError> {
        let mut bytes = Vec::new();
        input
            .take((HEADER_LENGTH + MAX_MANIFEST_BYTES + MAX_COMPRESSED_PAYLOAD_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| MapStorageError::Io(error.to_string()))?;
        if bytes.len() > HEADER_LENGTH + MAX_MANIFEST_BYTES + MAX_COMPRESSED_PAYLOAD_BYTES {
            return Err(MapStorageError::LimitExceeded("file length"));
        }
        Self::read(&bytes, known_tiles)
    }
}

pub struct MapProjectWriter {
    options: WriteOptions,
}

impl MapProjectWriter {
    pub const fn new(options: WriteOptions) -> Self {
        Self { options }
    }

    pub fn write(
        &self,
        project: &MapProject,
        known_tiles: &BTreeSet<AtomicTileId>,
    ) -> Result<Vec<u8>, MapStorageError> {
        project.validate(known_tiles)?;
        validate_dimensions(project.width, project.height)?;
        let raw = encode_project(project)?;
        if raw.len() > MAX_RAW_PAYLOAD_BYTES {
            return Err(MapStorageError::LimitExceeded("raw payload length"));
        }
        let checksum = *blake3::hash(&raw).as_bytes();
        let payload = zstd::stream::encode_all(raw.as_slice(), self.options.compression_level)
            .map_err(|error| MapStorageError::Io(error.to_string()))?;
        if payload.len() > MAX_COMPRESSED_PAYLOAD_BYTES {
            return Err(MapStorageError::LimitExceeded("compressed payload length"));
        }
        let manifest = encode_manifest(project, raw.len(), payload.len())?;
        if manifest.len() > MAX_MANIFEST_BYTES {
            return Err(MapStorageError::LimitExceeded("manifest length"));
        }
        let mut result = Vec::with_capacity(HEADER_LENGTH + manifest.len() + payload.len());
        result.extend_from_slice(&encode_header(
            manifest.len(),
            payload.len(),
            raw.len(),
            checksum,
        )?);
        result.extend_from_slice(&manifest);
        result.extend_from_slice(&payload);
        Ok(result)
    }

    pub fn write_to<W: Write>(
        &self,
        mut output: W,
        project: &MapProject,
        known_tiles: &BTreeSet<AtomicTileId>,
    ) -> Result<(), MapStorageError> {
        let bytes = self.write(project, known_tiles)?;
        output
            .write_all(&bytes)
            .map_err(|error| MapStorageError::Io(error.to_string()))
    }
}

impl Default for MapProjectWriter {
    fn default() -> Self {
        Self::new(WriteOptions::default())
    }
}

struct Header {
    payload_len: usize,
    raw_payload_len: usize,
    checksum: [u8; 32],
}

struct Container<'a> {
    header: Header,
    manifest: &'a [u8],
    payload: &'a [u8],
}

fn parse_container(input: &[u8]) -> Result<Container<'_>, MapStorageError> {
    if input.len() < HEADER_LENGTH {
        return Err(MapStorageError::Truncated);
    }
    if input[..4] != MAGIC {
        return Err(MapStorageError::BadMagic);
    }
    let version = u16::from_le_bytes([input[4], input[5]]);
    if version != CONTAINER_VERSION {
        return Err(MapStorageError::UnsupportedContainerVersion(version));
    }
    if u16::from_le_bytes([input[6], input[7]]) as usize != HEADER_LENGTH {
        return Err(MapStorageError::InvalidHeader("header length"));
    }
    if input[8] != COMPRESSION_ZSTD {
        return Err(MapStorageError::InvalidHeader("compression"));
    }
    if input[9..12] != [0, 0, 0] {
        return Err(MapStorageError::InvalidHeader("reserved bytes"));
    }
    let manifest_len = usize_from_u32(read_u32_at(input, 12)?)
        .map_err(|_| MapStorageError::LimitExceeded("manifest length"))?;
    let payload_len = usize_from_u64(read_u64_at(input, 16)?)
        .map_err(|_| MapStorageError::LimitExceeded("compressed payload length"))?;
    let raw_payload_len = usize_from_u64(read_u64_at(input, 24)?)
        .map_err(|_| MapStorageError::LimitExceeded("raw payload length"))?;
    if manifest_len > MAX_MANIFEST_BYTES {
        return Err(MapStorageError::LimitExceeded("manifest length"));
    }
    if payload_len > MAX_COMPRESSED_PAYLOAD_BYTES {
        return Err(MapStorageError::LimitExceeded("compressed payload length"));
    }
    if raw_payload_len > MAX_RAW_PAYLOAD_BYTES {
        return Err(MapStorageError::LimitExceeded("raw payload length"));
    }
    let expected = HEADER_LENGTH
        .checked_add(manifest_len)
        .and_then(|length| length.checked_add(payload_len))
        .ok_or(MapStorageError::InvalidHeader("container length overflow"))?;
    if expected != input.len() {
        return Err(MapStorageError::Truncated);
    }
    let mut checksum = [0; 32];
    checksum.copy_from_slice(&input[32..64]);
    Ok(Container {
        header: Header {
            payload_len,
            raw_payload_len,
            checksum,
        },
        manifest: &input[HEADER_LENGTH..HEADER_LENGTH + manifest_len],
        payload: &input[HEADER_LENGTH + manifest_len..],
    })
}

fn encode_header(
    manifest_len: usize,
    payload_len: usize,
    raw_payload_len: usize,
    checksum: [u8; 32],
) -> Result<[u8; HEADER_LENGTH], MapStorageError> {
    let mut header = [0; HEADER_LENGTH];
    header[..4].copy_from_slice(&MAGIC);
    header[4..6].copy_from_slice(&CONTAINER_VERSION.to_le_bytes());
    header[6..8].copy_from_slice(&(HEADER_LENGTH as u16).to_le_bytes());
    header[8] = COMPRESSION_ZSTD;
    header[12..16].copy_from_slice(&u32_from_usize(manifest_len, "manifest length")?.to_le_bytes());
    header[16..24]
        .copy_from_slice(&u64_from_usize(payload_len, "compressed payload length")?.to_le_bytes());
    header[24..32]
        .copy_from_slice(&u64_from_usize(raw_payload_len, "raw payload length")?.to_le_bytes());
    header[32..64].copy_from_slice(&checksum);
    Ok(header)
}

fn parse_manifest(input: &[u8], header: &Header) -> Result<MapProjectMetadata, MapStorageError> {
    let mut cursor = Cursor::new(input);
    if cursor.u16()? != MANIFEST_VERSION {
        return Err(MapStorageError::InvalidManifest("version"));
    }
    let document_format = cursor.string().map_err(manifest_error)?;
    let map_id = cursor.string().map_err(manifest_error)?;
    let tile_size = TilePixelSize::new(cursor.u16()?, cursor.u16()?);
    let width = cursor.u16()?;
    let height = cursor.u16()?;
    validate_dimensions(width, height)?;
    if document_format.trim().is_empty() || map_id.trim().is_empty() {
        return Err(MapStorageError::InvalidManifest("empty identifier"));
    }
    if tile_size.width == 0 || tile_size.height == 0 {
        return Err(MapStorageError::InvalidManifest("tile size"));
    }
    let cell_count = cursor.u32()?;
    if usize_from_u32(cell_count).map_err(|_| MapStorageError::InvalidManifest("cell count"))?
        != usize::from(width) * usize::from(height)
    {
        return Err(MapStorageError::InvalidManifest("cell count"));
    }
    let material_count = cursor.u32()?;
    let atomic_tile_count = cursor.u32()?;
    let actor_count = cursor.u32()?;
    let event_count = cursor.u32()?;
    if cursor.u16()? != PAYLOAD_SCHEMA_VERSION {
        return Err(MapStorageError::InvalidManifest("payload schema version"));
    }
    cursor.finish().map_err(manifest_error)?;
    Ok(MapProjectMetadata {
        container_version: CONTAINER_VERSION,
        document_format,
        map_id,
        tile_size,
        width,
        height,
        cell_count,
        material_count,
        atomic_tile_count,
        actor_count,
        event_count,
        compression: Compression::Zstd,
        compressed_payload_bytes: header.payload_len as u64,
        raw_payload_bytes: header.raw_payload_len as u64,
        payload_checksum: header.checksum,
    })
}

fn encode_manifest(
    project: &MapProject,
    raw_len: usize,
    compressed_len: usize,
) -> Result<Vec<u8>, MapStorageError> {
    let mut atomic_ids = BTreeSet::new();
    for material in &project.materials {
        for layer in &material.layers {
            atomic_ids.insert(layer.as_str());
        }
    }
    let event_count = project
        .event_cells
        .iter()
        .filter(|event| event.is_some())
        .count();
    let cell_count = project
        .width
        .checked_mul(project.height)
        .ok_or(MapStorageError::InvalidManifest("cell count overflow"))?;
    let mut manifest = Vec::new();
    push_u16(&mut manifest, MANIFEST_VERSION);
    push_string(&mut manifest, &project.format_version)?;
    push_string(&mut manifest, project.id.as_str())?;
    push_u16(&mut manifest, project.tile_size.width);
    push_u16(&mut manifest, project.tile_size.height);
    push_u16(&mut manifest, project.width);
    push_u16(&mut manifest, project.height);
    push_u32(&mut manifest, u32::from(cell_count));
    push_u32(
        &mut manifest,
        u32_from_usize(project.materials.len(), "material count")?,
    );
    push_u32(
        &mut manifest,
        u32_from_usize(atomic_ids.len(), "atomic tile count")?,
    );
    push_u32(
        &mut manifest,
        u32_from_usize(project.actors.len(), "actor count")?,
    );
    push_u32(&mut manifest, u32_from_usize(event_count, "event count")?);
    push_u16(&mut manifest, PAYLOAD_SCHEMA_VERSION);
    if raw_len > MAX_RAW_PAYLOAD_BYTES || compressed_len > MAX_COMPRESSED_PAYLOAD_BYTES {
        return Err(MapStorageError::LimitExceeded("payload length"));
    }
    Ok(manifest)
}

fn decompress(payload: &[u8], header: &Header) -> Result<Vec<u8>, MapStorageError> {
    let decoder = zstd::stream::read::Decoder::new(payload)
        .map_err(|error| MapStorageError::DecompressionFailed(error.to_string()))?;
    let mut raw = Vec::with_capacity(header.raw_payload_len);
    decoder
        .take(header.raw_payload_len as u64 + 1)
        .read_to_end(&mut raw)
        .map_err(|error| MapStorageError::DecompressionFailed(error.to_string()))?;
    if raw.len() != header.raw_payload_len {
        return Err(MapStorageError::InvalidPayload(
            "uncompressed payload length",
        ));
    }
    if *blake3::hash(&raw).as_bytes() != header.checksum {
        return Err(MapStorageError::ChecksumMismatch);
    }
    Ok(raw)
}

fn encode_project(project: &MapProject) -> Result<Vec<u8>, MapStorageError> {
    let strings = string_table(project);
    let indexes = strings
        .iter()
        .enumerate()
        .map(|(index, value)| (value.as_str(), index))
        .collect::<BTreeMap<_, _>>();
    let sections = [
        (SECTION_STRINGS, encode_strings(&strings)?),
        (
            SECTION_MATERIALS,
            encode_materials(&project.materials, &indexes)?,
        ),
        (SECTION_VISUAL, encode_visual(project)?),
        (SECTION_COLLISION, encode_collision(project)?),
        (SECTION_EVENTS, encode_events(project)?),
        (SECTION_ENTITIES, encode_entities(project, &indexes)?),
    ];
    let mut payload = Vec::new();
    push_u16(&mut payload, PAYLOAD_SCHEMA_VERSION);
    push_u16(&mut payload, sections.len() as u16);
    for (id, section) in sections {
        payload.push(id);
        push_u32(
            &mut payload,
            u32_from_usize(section.len(), "section length")?,
        );
        payload.extend_from_slice(&section);
    }
    Ok(payload)
}

fn string_table(project: &MapProject) -> Vec<String> {
    let mut strings = BTreeSet::new();
    for material in &project.materials {
        strings.insert(material.id.as_str().to_owned());
        for layer in &material.layers {
            strings.insert(layer.as_str().to_owned());
        }
    }
    for actor in &project.actors {
        strings.insert(actor.id.as_str().to_owned());
        strings.insert(actor.appearance.as_str().to_owned());
    }
    strings.into_iter().collect()
}

fn encode_strings(strings: &[String]) -> Result<Vec<u8>, MapStorageError> {
    let mut result = Vec::new();
    push_u32(&mut result, u32_from_usize(strings.len(), "string count")?);
    for value in strings {
        push_string(&mut result, value)?;
    }
    Ok(result)
}

fn encode_materials(
    materials: &[CompositeTile],
    indexes: &BTreeMap<&str, usize>,
) -> Result<Vec<u8>, MapStorageError> {
    let mut result = Vec::new();
    push_u32(
        &mut result,
        u32_from_usize(materials.len(), "material count")?,
    );
    for material in materials {
        push_u32(&mut result, string_index(indexes, material.id.as_str())?);
        push_u32(
            &mut result,
            u32_from_usize(material.layers.len(), "layer count")?,
        );
        for layer in &material.layers {
            push_u32(&mut result, string_index(indexes, layer.as_str())?);
        }
    }
    Ok(result)
}

fn encode_visual(project: &MapProject) -> Result<Vec<u8>, MapStorageError> {
    let index_width = index_width(project.materials.len())?;
    let mut result = vec![index_width];
    for row in 0..usize::from(project.height) {
        let start = row * usize::from(project.width);
        let values = project.visual_cells[start..start + usize::from(project.width)]
            .iter()
            .map(|cell| visual_index(project, cell))
            .collect::<Result<Vec<_>, _>>()?;
        let rle = encode_rle_indexes(&values, index_width)?;
        if rle.len() < values.len() * usize::from(index_width) {
            result.push(1);
            result.extend_from_slice(&rle);
        } else {
            result.push(0);
            for value in values {
                push_index(&mut result, value, index_width)?;
            }
        }
    }
    Ok(result)
}

fn visual_index(project: &MapProject, cell: &VisualCell) -> Result<u32, MapStorageError> {
    match &cell.material {
        None => Ok(0),
        Some(id) => project
            .materials
            .iter()
            .position(|material| &material.id == id)
            .and_then(|index| u32::try_from(index + 1).ok())
            .ok_or(MapStorageError::InvalidPayload("visual material index")),
    }
}

fn encode_rle_indexes(values: &[u32], index_width: u8) -> Result<Vec<u8>, MapStorageError> {
    let mut runs = Vec::new();
    let mut start = 0;
    while start < values.len() {
        let value = values[start];
        let mut end = start + 1;
        while end < values.len() && values[end] == value {
            end += 1;
        }
        runs.push((end - start, value));
        start = end;
    }
    let mut result = Vec::new();
    push_u16(&mut result, u16_from_usize(runs.len(), "visual run count")?);
    for (length, value) in runs {
        push_u16(&mut result, u16_from_usize(length, "visual run length")?);
        push_index(&mut result, value, index_width)?;
    }
    Ok(result)
}

fn encode_collision(project: &MapProject) -> Result<Vec<u8>, MapStorageError> {
    let bitset = encode_collision_bitset(&project.collision_cells);
    let rle = encode_collision_rle(project)?;
    if rle.len() < bitset.len() {
        let mut result = vec![1];
        result.extend_from_slice(&rle);
        Ok(result)
    } else {
        let mut result = vec![0];
        result.extend_from_slice(&bitset);
        Ok(result)
    }
}

fn encode_collision_bitset(cells: &[Collision]) -> Vec<u8> {
    let mut result = vec![0; cells.len().div_ceil(8)];
    for (index, collision) in cells.iter().enumerate() {
        if *collision == Collision::Blocked {
            result[index / 8] |= 1 << (index % 8);
        }
    }
    result
}

fn encode_collision_rle(project: &MapProject) -> Result<Vec<u8>, MapStorageError> {
    let mut result = Vec::new();
    for row in 0..usize::from(project.height) {
        let start = row * usize::from(project.width);
        let values = &project.collision_cells[start..start + usize::from(project.width)];
        let mut runs = Vec::new();
        let mut start = 0;
        while start < values.len() {
            let value = values[start];
            let mut end = start + 1;
            while end < values.len() && values[end] == value {
                end += 1;
            }
            runs.push((end - start, value));
            start = end;
        }
        push_u16(
            &mut result,
            u16_from_usize(runs.len(), "collision run count")?,
        );
        for (length, value) in runs {
            push_u16(&mut result, u16_from_usize(length, "collision run length")?);
            result.push(collision_code(value));
        }
    }
    Ok(result)
}

fn encode_events(project: &MapProject) -> Result<Vec<u8>, MapStorageError> {
    let entries = project
        .event_cells
        .iter()
        .enumerate()
        .filter_map(|(index, event)| event.map(|event| (index, event)))
        .collect::<Vec<_>>();
    let mut result = Vec::new();
    push_u32(&mut result, u32_from_usize(entries.len(), "event count")?);
    let mut previous = 0usize;
    for (offset, (index, event)) in entries.iter().enumerate() {
        let delta = if offset == 0 {
            *index
        } else {
            *index - previous
        };
        push_u32(&mut result, u32_from_usize(delta, "event index delta")?);
        result.push(event_code(*event));
        previous = *index;
    }
    Ok(result)
}

fn encode_entities(
    project: &MapProject,
    indexes: &BTreeMap<&str, usize>,
) -> Result<Vec<u8>, MapStorageError> {
    let mut result = Vec::new();
    push_position(&mut result, project.player_spawn);
    push_u32(
        &mut result,
        u32_from_usize(project.actors.len(), "actor count")?,
    );
    for actor in &project.actors {
        push_u32(&mut result, string_index(indexes, actor.id.as_str())?);
        push_position(&mut result, actor.position);
        result.push(direction_code(actor.facing));
        push_u32(
            &mut result,
            string_index(indexes, actor.appearance.as_str())?,
        );
    }
    Ok(result)
}

fn decode_project(
    raw: &[u8],
    metadata: &MapProjectMetadata,
    known_tiles: &BTreeSet<AtomicTileId>,
) -> Result<MapProject, MapStorageError> {
    let sections = parse_sections(raw)?;
    let strings = decode_strings(required_section(&sections, SECTION_STRINGS)?)?;
    let materials = decode_materials(required_section(&sections, SECTION_MATERIALS)?, &strings)?;
    if materials.len()
        != usize_from_u32(metadata.material_count)
            .map_err(|_| MapStorageError::InvalidManifest("material count"))?
    {
        return Err(MapStorageError::InvalidPayload(
            "material count does not match manifest",
        ));
    }
    let visual_cells = decode_visual(
        required_section(&sections, SECTION_VISUAL)?,
        metadata.width,
        metadata.height,
        &materials,
    )?;
    let collision_cells = decode_collision(
        required_section(&sections, SECTION_COLLISION)?,
        metadata.width,
        metadata.height,
    )?;
    let event_cells = decode_events(
        required_section(&sections, SECTION_EVENTS)?,
        usize_from_u32(metadata.cell_count)
            .map_err(|_| MapStorageError::InvalidManifest("cell count"))?,
    )?;
    let (player_spawn, actors) =
        decode_entities(required_section(&sections, SECTION_ENTITIES)?, &strings)?;
    if actors.len()
        != usize_from_u32(metadata.actor_count)
            .map_err(|_| MapStorageError::InvalidManifest("actor count"))?
    {
        return Err(MapStorageError::InvalidPayload(
            "actor count does not match manifest",
        ));
    }
    if event_cells.iter().filter(|event| event.is_some()).count()
        != usize_from_u32(metadata.event_count)
            .map_err(|_| MapStorageError::InvalidManifest("event count"))?
    {
        return Err(MapStorageError::InvalidPayload(
            "event count does not match manifest",
        ));
    }
    let atomic_count = materials
        .iter()
        .flat_map(|material| material.layers.iter())
        .collect::<BTreeSet<_>>()
        .len();
    if atomic_count
        != usize_from_u32(metadata.atomic_tile_count)
            .map_err(|_| MapStorageError::InvalidManifest("atomic tile count"))?
    {
        return Err(MapStorageError::InvalidPayload(
            "atomic tile count does not match manifest",
        ));
    }
    let project = MapProject {
        format_version: metadata.document_format.clone(),
        id: MapProjectId::new(metadata.map_id.clone())?,
        tile_size: metadata.tile_size,
        width: metadata.width,
        height: metadata.height,
        materials,
        visual_cells,
        collision_cells,
        event_cells,
        player_spawn,
        actors,
    };
    project.validate(known_tiles)?;
    Ok(project)
}

fn parse_sections(raw: &[u8]) -> Result<BTreeMap<u8, &[u8]>, MapStorageError> {
    let mut cursor = Cursor::new(raw);
    if cursor.u16()? != PAYLOAD_SCHEMA_VERSION {
        return Err(MapStorageError::InvalidPayload("schema version"));
    }
    let count = usize::from(cursor.u16()?);
    if count > 64 {
        return Err(MapStorageError::InvalidPayload("section count"));
    }
    let mut sections = BTreeMap::new();
    for _ in 0..count {
        let id = cursor.u8()?;
        let length = usize_from_u32(cursor.u32()?)
            .map_err(|_| MapStorageError::InvalidPayload("section length"))?;
        let bytes = cursor.bytes(length)?;
        if sections.insert(id, bytes).is_some() {
            return Err(MapStorageError::InvalidPayload("duplicate section"));
        }
    }
    cursor.finish()?;
    Ok(sections)
}

fn required_section<'a>(
    sections: &'a BTreeMap<u8, &'a [u8]>,
    id: u8,
) -> Result<&'a [u8], MapStorageError> {
    sections
        .get(&id)
        .copied()
        .ok_or(MapStorageError::InvalidPayload("missing required section"))
}

fn decode_strings(input: &[u8]) -> Result<Vec<String>, MapStorageError> {
    let mut cursor = Cursor::new(input);
    let count = usize_from_u32(cursor.u32()?)
        .map_err(|_| MapStorageError::InvalidPayload("string count"))?;
    if count > MAX_CELL_COUNT {
        return Err(MapStorageError::LimitExceeded("string count"));
    }
    let mut strings = Vec::with_capacity(count);
    let mut previous = None;
    for _ in 0..count {
        let value = cursor.string()?;
        if previous
            .as_ref()
            .is_some_and(|previous: &String| previous >= &value)
        {
            return Err(MapStorageError::InvalidPayload("string table order"));
        }
        previous = Some(value.clone());
        strings.push(value);
    }
    cursor.finish()?;
    Ok(strings)
}

fn decode_materials(
    input: &[u8],
    strings: &[String],
) -> Result<Vec<CompositeTile>, MapStorageError> {
    let mut cursor = Cursor::new(input);
    let count = usize_from_u32(cursor.u32()?)
        .map_err(|_| MapStorageError::InvalidPayload("material count"))?;
    if count > MAX_CELL_COUNT {
        return Err(MapStorageError::LimitExceeded("material count"));
    }
    let mut materials = Vec::with_capacity(count);
    for _ in 0..count {
        let id = CompositeTileId::new(string_at(strings, cursor.u32()?)?.to_owned())?;
        let layer_count = usize_from_u32(cursor.u32()?)
            .map_err(|_| MapStorageError::InvalidPayload("layer count"))?;
        if layer_count == 0 || layer_count > MAX_CELL_COUNT {
            return Err(MapStorageError::InvalidPayload("layer count"));
        }
        let mut layers = Vec::with_capacity(layer_count);
        for _ in 0..layer_count {
            layers.push(AtomicTileId::new(
                string_at(strings, cursor.u32()?)?.to_owned(),
            )?);
        }
        materials.push(CompositeTile::new(id, layers));
    }
    cursor.finish()?;
    Ok(materials)
}

fn decode_visual(
    input: &[u8],
    width: u16,
    height: u16,
    materials: &[CompositeTile],
) -> Result<Vec<VisualCell>, MapStorageError> {
    let mut cursor = Cursor::new(input);
    let index_width = cursor.u8()?;
    if !matches!(index_width, 1 | 2 | 4) {
        return Err(MapStorageError::InvalidPayload("visual index width"));
    }
    let mut cells = Vec::with_capacity(usize::from(width) * usize::from(height));
    for _ in 0..height {
        match cursor.u8()? {
            0 => {
                for _ in 0..width {
                    cells.push(VisualCell::new(material_for_index(
                        materials,
                        cursor.index(index_width)?,
                    )?));
                }
            }
            1 => {
                let run_count = usize::from(cursor.u16()?);
                if run_count == 0 {
                    return Err(MapStorageError::InvalidPayload("visual run count"));
                }
                let mut total = 0usize;
                for _ in 0..run_count {
                    let length = usize::from(cursor.u16()?);
                    if length == 0 {
                        return Err(MapStorageError::InvalidPayload("visual run length"));
                    }
                    let material = material_for_index(materials, cursor.index(index_width)?)?;
                    total = total
                        .checked_add(length)
                        .ok_or(MapStorageError::InvalidPayload("visual run length"))?;
                    if total > usize::from(width) {
                        return Err(MapStorageError::InvalidPayload("visual row length"));
                    }
                    cells.extend(
                        std::iter::repeat_with(|| VisualCell::new(material.clone())).take(length),
                    );
                }
                if total != usize::from(width) {
                    return Err(MapStorageError::InvalidPayload("visual row length"));
                }
            }
            _ => return Err(MapStorageError::InvalidPayload("visual row encoding")),
        }
    }
    cursor.finish()?;
    Ok(cells)
}

fn material_for_index(
    materials: &[CompositeTile],
    index: u32,
) -> Result<Option<CompositeTileId>, MapStorageError> {
    if index == 0 {
        return Ok(None);
    }
    materials
        .get(
            usize_from_u32(index - 1)
                .map_err(|_| MapStorageError::InvalidPayload("visual material index"))?,
        )
        .map(|material| Some(material.id.clone()))
        .ok_or(MapStorageError::InvalidPayload("visual material index"))
}

fn decode_collision(
    input: &[u8],
    width: u16,
    height: u16,
) -> Result<Vec<Collision>, MapStorageError> {
    let expected = usize::from(width) * usize::from(height);
    let mut cursor = Cursor::new(input);
    let cells = match cursor.u8()? {
        0 => {
            let bytes = cursor.bytes(expected.div_ceil(8))?;
            (0..expected)
                .map(|index| {
                    if bytes[index / 8] & (1 << (index % 8)) != 0 {
                        Collision::Blocked
                    } else {
                        Collision::Walkable
                    }
                })
                .collect()
        }
        1 => {
            let mut cells = Vec::with_capacity(expected);
            for _ in 0..height {
                let run_count = usize::from(cursor.u16()?);
                if run_count == 0 {
                    return Err(MapStorageError::InvalidPayload("collision run count"));
                }
                let mut total = 0usize;
                for _ in 0..run_count {
                    let length = usize::from(cursor.u16()?);
                    if length == 0 {
                        return Err(MapStorageError::InvalidPayload("collision run length"));
                    }
                    let collision = collision_from_code(cursor.u8()?)?;
                    total = total
                        .checked_add(length)
                        .ok_or(MapStorageError::InvalidPayload("collision run length"))?;
                    if total > usize::from(width) {
                        return Err(MapStorageError::InvalidPayload("collision row length"));
                    }
                    cells.extend(std::iter::repeat_n(collision, length));
                }
                if total != usize::from(width) {
                    return Err(MapStorageError::InvalidPayload("collision row length"));
                }
            }
            cells
        }
        _ => return Err(MapStorageError::InvalidPayload("collision encoding")),
    };
    cursor.finish()?;
    Ok(cells)
}

fn decode_events(
    input: &[u8],
    cell_count: usize,
) -> Result<Vec<Option<MapEventKind>>, MapStorageError> {
    let mut cursor = Cursor::new(input);
    let count = usize_from_u32(cursor.u32()?)
        .map_err(|_| MapStorageError::InvalidPayload("event count"))?;
    if count > cell_count {
        return Err(MapStorageError::InvalidPayload("event count"));
    }
    let mut cells = vec![None; cell_count];
    let mut previous = None;
    for _ in 0..count {
        let delta = usize_from_u32(cursor.u32()?)
            .map_err(|_| MapStorageError::InvalidPayload("event index"))?;
        let index = previous.map_or(delta, |previous: usize| {
            previous.checked_add(delta).unwrap_or(cell_count)
        });
        if index >= cell_count || previous.is_some_and(|previous| index <= previous) {
            return Err(MapStorageError::InvalidPayload("event index"));
        }
        cells[index] = Some(event_from_code(cursor.u8()?)?);
        previous = Some(index);
    }
    cursor.finish()?;
    Ok(cells)
}

fn decode_entities(
    input: &[u8],
    strings: &[String],
) -> Result<(TilePosition, Vec<MapActor>), MapStorageError> {
    let mut cursor = Cursor::new(input);
    let player_spawn = cursor.position()?;
    let count = usize_from_u32(cursor.u32()?)
        .map_err(|_| MapStorageError::InvalidPayload("actor count"))?;
    if count > MAX_CELL_COUNT {
        return Err(MapStorageError::LimitExceeded("actor count"));
    }
    let mut actors = Vec::with_capacity(count);
    for _ in 0..count {
        let id = MapActorId::new(string_at(strings, cursor.u32()?)?.to_owned())?;
        let position = cursor.position()?;
        let facing = direction_from_code(cursor.u8()?)?;
        let appearance = CharacterAppearanceId::new(string_at(strings, cursor.u32()?)?.to_owned())?;
        actors.push(MapActor::new(id, position, facing, appearance));
    }
    cursor.finish()?;
    Ok((player_spawn, actors))
}

fn validate_dimensions(width: u16, height: u16) -> Result<(), MapStorageError> {
    if width == 0 || height == 0 || width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(MapStorageError::LimitExceeded("map dimensions"));
    }
    if usize::from(width) * usize::from(height) > MAX_CELL_COUNT {
        return Err(MapStorageError::LimitExceeded("cell count"));
    }
    Ok(())
}

fn event_code(event: MapEventKind) -> u8 {
    match event {
        MapEventKind::Encounter => 1,
    }
}

fn event_from_code(value: u8) -> Result<MapEventKind, MapStorageError> {
    match value {
        1 => Ok(MapEventKind::Encounter),
        _ => Err(MapStorageError::InvalidPayload("event kind")),
    }
}

fn collision_code(collision: Collision) -> u8 {
    match collision {
        Collision::Walkable => 0,
        Collision::Blocked => 1,
    }
}

fn collision_from_code(value: u8) -> Result<Collision, MapStorageError> {
    match value {
        0 => Ok(Collision::Walkable),
        1 => Ok(Collision::Blocked),
        _ => Err(MapStorageError::InvalidPayload("collision value")),
    }
}

fn direction_code(direction: MapDirection) -> u8 {
    match direction {
        MapDirection::Up => 0,
        MapDirection::Down => 1,
        MapDirection::Left => 2,
        MapDirection::Right => 3,
    }
}

fn direction_from_code(value: u8) -> Result<MapDirection, MapStorageError> {
    match value {
        0 => Ok(MapDirection::Up),
        1 => Ok(MapDirection::Down),
        2 => Ok(MapDirection::Left),
        3 => Ok(MapDirection::Right),
        _ => Err(MapStorageError::InvalidPayload("actor direction")),
    }
}

fn index_width(material_count: usize) -> Result<u8, MapStorageError> {
    let indexed = material_count
        .checked_add(1)
        .ok_or(MapStorageError::LimitExceeded("material count"))?;
    if indexed <= usize::from(u8::MAX) + 1 {
        Ok(1)
    } else if indexed <= usize::from(u16::MAX) + 1 {
        Ok(2)
    } else {
        Ok(4)
    }
}

fn push_index(output: &mut Vec<u8>, value: u32, width: u8) -> Result<(), MapStorageError> {
    match width {
        1 => output.push(value as u8),
        2 => output.extend_from_slice(&(value as u16).to_le_bytes()),
        4 => output.extend_from_slice(&value.to_le_bytes()),
        _ => return Err(MapStorageError::InvalidPayload("index width")),
    }
    Ok(())
}

fn string_index(indexes: &BTreeMap<&str, usize>, value: &str) -> Result<u32, MapStorageError> {
    indexes
        .get(value)
        .copied()
        .ok_or(MapStorageError::InvalidPayload(
            "missing string table entry",
        ))
        .and_then(|index| u32_from_usize(index, "string index"))
}

fn string_at(strings: &[String], index: u32) -> Result<&str, MapStorageError> {
    strings
        .get(usize_from_u32(index).map_err(|_| MapStorageError::InvalidPayload("string index"))?)
        .map(String::as_str)
        .ok_or(MapStorageError::InvalidPayload("string index"))
}

fn push_position(output: &mut Vec<u8>, position: TilePosition) {
    push_u16(output, position.x());
    push_u16(output, position.y());
}

fn push_string(output: &mut Vec<u8>, value: &str) -> Result<(), MapStorageError> {
    let bytes = value.as_bytes();
    push_u16(output, u16_from_usize(bytes.len(), "string length")?);
    output.extend_from_slice(bytes);
    Ok(())
}

fn push_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn u16_from_usize(value: usize, field: &'static str) -> Result<u16, MapStorageError> {
    u16::try_from(value).map_err(|_| MapStorageError::LimitExceeded(field))
}

fn u32_from_usize(value: usize, field: &'static str) -> Result<u32, MapStorageError> {
    u32::try_from(value).map_err(|_| MapStorageError::LimitExceeded(field))
}

fn u64_from_usize(value: usize, field: &'static str) -> Result<u64, MapStorageError> {
    u64::try_from(value).map_err(|_| MapStorageError::LimitExceeded(field))
}

fn usize_from_u32(value: u32) -> Result<usize, ()> {
    usize::try_from(value).map_err(|_| ())
}

fn usize_from_u64(value: u64) -> Result<usize, ()> {
    usize::try_from(value).map_err(|_| ())
}

fn read_u32_at(input: &[u8], offset: usize) -> Result<u32, MapStorageError> {
    let end = offset.checked_add(4).ok_or(MapStorageError::Truncated)?;
    let bytes = input.get(offset..end).ok_or(MapStorageError::Truncated)?;
    let bytes = bytes.try_into().map_err(|_| MapStorageError::Truncated)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64_at(input: &[u8], offset: usize) -> Result<u64, MapStorageError> {
    let end = offset.checked_add(8).ok_or(MapStorageError::Truncated)?;
    let bytes = input.get(offset..end).ok_or(MapStorageError::Truncated)?;
    let bytes = bytes.try_into().map_err(|_| MapStorageError::Truncated)?;
    Ok(u64::from_le_bytes(bytes))
}

fn manifest_error(error: MapStorageError) -> MapStorageError {
    match error {
        MapStorageError::Truncated => MapStorageError::InvalidManifest("truncated"),
        other => other,
    }
}

struct Cursor<'a> {
    input: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    const fn new(input: &'a [u8]) -> Self {
        Self { input, offset: 0 }
    }

    fn bytes(&mut self, length: usize) -> Result<&'a [u8], MapStorageError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(MapStorageError::Truncated)?;
        let bytes = self
            .input
            .get(self.offset..end)
            .ok_or(MapStorageError::Truncated)?;
        self.offset = end;
        Ok(bytes)
    }

    fn u8(&mut self) -> Result<u8, MapStorageError> {
        Ok(self.bytes(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, MapStorageError> {
        let bytes = self
            .bytes(2)?
            .try_into()
            .map_err(|_| MapStorageError::Truncated)?;
        Ok(u16::from_le_bytes(bytes))
    }

    fn u32(&mut self) -> Result<u32, MapStorageError> {
        let bytes = self
            .bytes(4)?
            .try_into()
            .map_err(|_| MapStorageError::Truncated)?;
        Ok(u32::from_le_bytes(bytes))
    }

    fn string(&mut self) -> Result<String, MapStorageError> {
        let length = usize::from(self.u16()?);
        String::from_utf8(self.bytes(length)?.to_vec())
            .map_err(|_| MapStorageError::InvalidPayload("UTF-8 string"))
    }

    fn index(&mut self, width: u8) -> Result<u32, MapStorageError> {
        match width {
            1 => Ok(u32::from(self.u8()?)),
            2 => Ok(u32::from(self.u16()?)),
            4 => self.u32(),
            _ => Err(MapStorageError::InvalidPayload("index width")),
        }
    }

    fn position(&mut self) -> Result<TilePosition, MapStorageError> {
        Ok(TilePosition::new(self.u16()?, self.u16()?))
    }

    fn finish(self) -> Result<(), MapStorageError> {
        if self.offset == self.input.len() {
            Ok(())
        } else {
            Err(MapStorageError::InvalidPayload("trailing bytes"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tile(value: &str) -> AtomicTileId {
        AtomicTileId::new(value).unwrap()
    }

    fn fixture() -> (MapProject, BTreeSet<AtomicTileId>) {
        let ground = CompositeTile::new(
            CompositeTileId::new("ground").unwrap(),
            vec![tile("tile-ground")],
        );
        let wall = CompositeTile::new(
            CompositeTileId::new("wall").unwrap(),
            vec![tile("tile-wall")],
        );
        let mut project = MapProject::blank(
            MapProjectId::new("fixture").unwrap(),
            4,
            2,
            Some(ground.clone()),
        );
        project.materials.push(wall.clone());
        project.visual_cells[3] = VisualCell::new(Some(wall.id.clone()));
        project.collision_cells[3] = Collision::Blocked;
        project.event_cells[1] = Some(MapEventKind::Encounter);
        project.player_spawn = TilePosition::new(1, 1);
        project.actors.push(MapActor::new(
            MapActorId::new("guide").unwrap(),
            TilePosition::new(2, 1),
            MapDirection::Left,
            CharacterAppearanceId::new("hero").unwrap(),
        ));
        let known = [tile("tile-ground"), tile("tile-wall")]
            .into_iter()
            .collect();
        (project, known)
    }

    #[test]
    fn round_trips_and_inspects_a_valid_project() {
        let (project, known) = fixture();
        let bytes = MapProjectWriter::default().write(&project, &known).unwrap();
        let metadata = MapProjectReader::inspect(&bytes).unwrap();
        assert_eq!(metadata.map_id, "fixture");
        assert_eq!((metadata.width, metadata.height), (4, 2));
        assert_eq!(metadata.material_count, 2);
        assert_eq!(metadata.event_count, 1);
        assert_eq!(MapProjectReader::read(&bytes, &known).unwrap(), project);
    }

    #[test]
    fn output_is_deterministic() {
        let (project, known) = fixture();
        let writer = MapProjectWriter::default();
        assert_eq!(
            writer.write(&project, &known).unwrap(),
            writer.write(&project, &known).unwrap()
        );
    }

    #[test]
    fn detects_truncation_and_corruption() {
        let (project, known) = fixture();
        let bytes = MapProjectWriter::default().write(&project, &known).unwrap();
        assert!(matches!(
            MapProjectReader::read(&bytes[..20], &known),
            Err(MapStorageError::Truncated)
        ));
        let mut corrupt = bytes;
        let last = corrupt.len() - 1;
        corrupt[last] ^= 0xff;
        assert!(MapProjectReader::read(&corrupt, &known).is_err());
    }

    #[test]
    fn uses_rle_for_uniform_wide_rows() {
        let ground = CompositeTile::new(
            CompositeTileId::new("ground").unwrap(),
            vec![tile("tile-ground")],
        );
        let mut project =
            MapProject::blank(MapProjectId::new("uniform").unwrap(), 128, 1, Some(ground));
        project.player_spawn = TilePosition::new(0, 0);
        project.collision_cells.fill(Collision::Blocked);
        project.collision_cells[0] = Collision::Walkable;
        let known = [tile("tile-ground")].into_iter().collect();

        let raw = encode_project(&project).unwrap();
        let sections = parse_sections(&raw).unwrap();
        assert_eq!(required_section(&sections, SECTION_VISUAL).unwrap()[1], 1);
        assert_eq!(
            required_section(&sections, SECTION_COLLISION).unwrap()[0],
            1
        );

        let bytes = MapProjectWriter::default().write(&project, &known).unwrap();
        assert_eq!(MapProjectReader::read(&bytes, &known).unwrap(), project);
    }
}
