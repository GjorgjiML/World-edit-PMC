use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::Path;

use pumpkin_data::Block;
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_nbt::nbt_compress::{read_gzip_compound_tag, write_gzip_compound_tag};
use pumpkin_nbt::tag::NbtTag;
use pumpkin_util::math::vector3::Vector3;

use crate::state::ClipboardData;

/// Data version for Minecraft 1.21.11 (used when saving schematics).
const MC_DATA_VERSION: i32 = 4671;

/// Represents a loaded schematic.
pub struct SchematicData {
    pub width: u16,
    pub height: u16,
    pub length: u16,
    pub offset: Vector3<i32>,
    /// (relative position, block state id) - air blocks are excluded.
    pub blocks: Vec<(Vector3<i32>, u16)>,
}

// ============================================================================
// Block State String Parsing
// ============================================================================

/// Parse a block state string like "minecraft:oak_stairs[facing=north,half=bottom]"
/// into a block name and a list of property key-value pairs.
fn parse_block_state_string(s: &str) -> (&str, Vec<(&str, &str)>) {
    if let Some(bracket_start) = s.find('[') {
        let name = &s[..bracket_start];
        let bracket_end = s.rfind(']').unwrap_or(s.len());
        let props_str = &s[bracket_start + 1..bracket_end];
        let props: Vec<(&str, &str)> = props_str
            .split(',')
            .filter_map(|kv| {
                let mut parts = kv.splitn(2, '=');
                let key = parts.next()?.trim();
                let value = parts.next()?.trim();
                if key.is_empty() {
                    None
                } else {
                    Some((key, value))
                }
            })
            .collect();
        (name, props)
    } else {
        (s, Vec::new())
    }
}

/// Resolve a block state string from a schematic palette to a Pumpkin block state ID.
fn resolve_block_state(block_state_str: &str) -> Option<u16> {
    let (name, props) = parse_block_state_string(block_state_str);
    let block = Block::from_name(name)?;

    if props.is_empty() || block.states.len() <= 1 {
        // No properties or block doesn't have variants → use default state
        Some(block.default_state.id)
    } else {
        // Try to resolve with properties; fall back to default if it panics
        let result = std::panic::catch_unwind(|| {
            let block_props = block.from_properties(&props);
            block_props.to_state_id(block)
        });
        match result {
            Ok(state_id) => Some(state_id),
            Err(_) => {
                log::warn!(
                    "Failed to resolve properties for {block_state_str}, using default state"
                );
                Some(block.default_state.id)
            }
        }
    }
}

/// Build a block state string (for schematic palette) from a Pumpkin state ID.
fn build_block_state_string(state_id: u16) -> String {
    let block = Block::from_state_id(state_id);
    let name = format!("minecraft:{}", block.name);

    if let Some(props) = block.properties(state_id) {
        let prop_list: Vec<(&str, &str)> = props.to_props();
        if prop_list.is_empty() {
            name
        } else {
            let props_str: Vec<String> = prop_list
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect();
            format!("{name}[{}]", props_str.join(","))
        }
    } else {
        name
    }
}

// ============================================================================
// Varint Encoding / Decoding
// ============================================================================

/// Decode varint-encoded integers from a byte array.
fn decode_varints(data: &[u8], expected_count: usize) -> Result<Vec<i32>, String> {
    let mut result = Vec::with_capacity(expected_count);
    let mut i = 0;

    while i < data.len() && result.len() < expected_count {
        let mut value: i32 = 0;
        let mut bit_offset = 0;

        loop {
            if i >= data.len() {
                return Err("Unexpected end of varint data".to_string());
            }
            let byte = data[i] as i32;
            i += 1;

            value |= (byte & 0x7F) << bit_offset;
            bit_offset += 7;

            if byte & 0x80 == 0 {
                break;
            }
            if bit_offset >= 35 {
                return Err("Varint too large".to_string());
            }
        }

        result.push(value);
    }

    Ok(result)
}

/// Encode integers as a varint byte array.
fn encode_varints(values: &[i32]) -> Vec<u8> {
    let mut result = Vec::new();

    for &value in values {
        let mut v = value as u32;
        loop {
            let mut byte = (v & 0x7F) as u8;
            v >>= 7;
            if v != 0 {
                byte |= 0x80;
            }
            result.push(byte);
            if v == 0 {
                break;
            }
        }
    }

    result
}

// ============================================================================
// Load Schematic
// ============================================================================

/// Load a schematic from a `.schem` file. Supports both v2 and v3 formats.
pub fn load_schematic(path: &Path) -> Result<SchematicData, String> {
    // Read file into memory, then wrap in Cursor (read_gzip_compound_tag needs Read + Seek)
    let data = fs::read(path).map_err(|e| format!("Failed to read schematic file: {e}"))?;
    let root = read_gzip_compound_tag(Cursor::new(data))
        .map_err(|e| format!("Failed to parse NBT data: {e}"))?;

    // Detect version: v3 nests everything under "Schematic", v2 is flat
    let (version, data_root);
    if let Some(schematic) = root.get_compound("Schematic") {
        version = schematic.get_int("Version").unwrap_or(3);
        data_root = schematic;
    } else {
        version = root.get_int("Version").unwrap_or(2);
        data_root = &root;
    };

    log::info!("Loading schematic (version {version})");

    // Read dimensions
    let width = data_root
        .get_short("Width")
        .ok_or("Missing Width tag")? as u16;
    let height = data_root
        .get_short("Height")
        .ok_or("Missing Height tag")? as u16;
    let length = data_root
        .get_short("Length")
        .ok_or("Missing Length tag")? as u16;

    log::info!("Schematic dimensions: {width}x{height}x{length}");

    // Read offset (optional)
    let offset = if let Some(offset_arr) = data_root.get_int_array("Offset") {
        if offset_arr.len() >= 3 {
            Vector3::new(offset_arr[0], offset_arr[1], offset_arr[2])
        } else {
            Vector3::new(0, 0, 0)
        }
    } else {
        Vector3::new(0, 0, 0)
    };

    // Read palette and block data depending on version
    let (palette_compound, block_data_bytes) = if version >= 3 {
        // v3: nested under Blocks compound
        let blocks = data_root
            .get_compound("Blocks")
            .ok_or("Missing Blocks compound")?;
        let palette = blocks
            .get_compound("Palette")
            .ok_or("Missing Blocks.Palette compound")?;
        let data = blocks
            .get("Data")
            .and_then(|t| t.extract_byte_array())
            .ok_or("Missing Blocks.Data byte array")?;
        (palette, data)
    } else {
        // v2: flat in root
        let palette = data_root
            .get_compound("Palette")
            .ok_or("Missing Palette compound")?;
        let data = data_root
            .get("BlockData")
            .and_then(|t| t.extract_byte_array())
            .ok_or("Missing BlockData byte array")?;
        (palette, data)
    };

    // Build palette map: palette index → block state string
    let mut palette_map: HashMap<i32, String> = HashMap::new();
    for (name, tag) in &palette_compound.child_tags {
        if let NbtTag::Int(index) = tag {
            palette_map.insert(*index, name.clone());
        }
    }

    log::info!("Palette has {} entries", palette_map.len());

    // Decode varint block data
    let expected_blocks = (width as usize) * (height as usize) * (length as usize);
    let block_indices = decode_varints(block_data_bytes, expected_blocks)?;

    if block_indices.len() != expected_blocks {
        return Err(format!(
            "Block data count mismatch: expected {expected_blocks}, got {}",
            block_indices.len()
        ));
    }

    // Resolve palette entries to state IDs
    let air_state_id = Block::from_name("minecraft:air")
        .map(|b| b.default_state.id)
        .unwrap_or(0);

    let mut blocks = Vec::new();

    for (i, &palette_index) in block_indices.iter().enumerate() {
        // Schematic index: x + z * Width + y * Width * Length
        let y = (i / (width as usize * length as usize)) as i32;
        let remainder = i % (width as usize * length as usize);
        let z = (remainder / width as usize) as i32;
        let x = (remainder % width as usize) as i32;

        let state_id = if let Some(block_state_str) = palette_map.get(&palette_index) {
            resolve_block_state(block_state_str).unwrap_or_else(|| {
                log::warn!("Unknown block state: {block_state_str}, using air");
                air_state_id
            })
        } else {
            log::warn!("Palette index {palette_index} not found, using air");
            air_state_id
        };

        // Skip air blocks to save memory
        if state_id != air_state_id {
            blocks.push((
                Vector3::new(x + offset.x, y + offset.y, z + offset.z),
                state_id,
            ));
        }
    }

    log::info!(
        "Loaded schematic: {width}x{height}x{length}, {} non-air blocks",
        blocks.len()
    );

    Ok(SchematicData {
        width,
        height,
        length,
        offset,
        blocks,
    })
}

/// Convert a loaded schematic into clipboard data for pasting.
pub fn schematic_to_clipboard(schem: &SchematicData) -> ClipboardData {
    ClipboardData {
        blocks: schem.blocks.clone(),
    }
}

// ============================================================================
// Save Schematic
// ============================================================================

/// Save clipboard data as a `.schem` file (Sponge Schematic v3 format).
pub fn save_schematic(path: &Path, clipboard: &ClipboardData) -> Result<(), String> {
    if clipboard.blocks.is_empty() {
        return Err("Clipboard is empty".to_string());
    }

    // Calculate bounding box
    let mut min = clipboard.blocks[0].0;
    let mut max = clipboard.blocks[0].0;

    for (pos, _) in &clipboard.blocks {
        min.x = min.x.min(pos.x);
        min.y = min.y.min(pos.y);
        min.z = min.z.min(pos.z);
        max.x = max.x.max(pos.x);
        max.y = max.y.max(pos.y);
        max.z = max.z.max(pos.z);
    }

    let width = (max.x - min.x + 1) as u16;
    let height = (max.y - min.y + 1) as u16;
    let length = (max.z - min.z + 1) as u16;

    // Build palette
    let air_state_id = Block::from_name("minecraft:air")
        .map(|b| b.default_state.id)
        .unwrap_or(0);

    let mut palette: HashMap<u16, i32> = HashMap::new(); // state_id → palette_index
    let mut palette_names: HashMap<i32, String> = HashMap::new(); // palette_index → name
    let mut next_index: i32 = 0;

    // Air is always palette index 0
    palette.insert(air_state_id, 0);
    palette_names.insert(0, "minecraft:air".to_string());
    next_index += 1;

    for (_, state_id) in &clipboard.blocks {
        if !palette.contains_key(state_id) {
            palette.insert(*state_id, next_index);
            palette_names.insert(next_index, build_block_state_string(*state_id));
            next_index += 1;
        }
    }

    // Build block data grid (filled with air = palette index 0)
    let total_blocks = (width as usize) * (height as usize) * (length as usize);
    let mut block_data = vec![0i32; total_blocks];

    // Quick lookup for clipboard blocks
    let mut block_map: HashMap<(i32, i32, i32), u16> = HashMap::new();
    for (pos, state_id) in &clipboard.blocks {
        block_map.insert((pos.x, pos.y, pos.z), *state_id);
    }

    for y in 0..height as i32 {
        for z in 0..length as i32 {
            for x in 0..width as i32 {
                let world_x = x + min.x;
                let world_y = y + min.y;
                let world_z = z + min.z;

                let index =
                    x as usize + z as usize * width as usize + y as usize * width as usize * length as usize;

                if let Some(&state_id) = block_map.get(&(world_x, world_y, world_z)) {
                    if let Some(&palette_idx) = palette.get(&state_id) {
                        block_data[index] = palette_idx;
                    }
                }
            }
        }
    }

    // Encode as varints
    let encoded_data = encode_varints(&block_data);

    // Build NBT structure (v3)
    let mut palette_compound = NbtCompound::new();
    for (index, name) in &palette_names {
        palette_compound.put(name, NbtTag::Int(*index));
    }

    let mut blocks_compound = NbtCompound::new();
    blocks_compound.put_component("Palette", palette_compound);
    blocks_compound.put("Data", NbtTag::ByteArray(encoded_data.into_boxed_slice()));

    let mut schematic = NbtCompound::new();
    schematic.put_int("Version", 3);
    schematic.put_int("DataVersion", MC_DATA_VERSION);
    schematic.put_short("Width", width as i16);
    schematic.put_short("Height", height as i16);
    schematic.put_short("Length", length as i16);
    schematic.put("Offset", NbtTag::IntArray(vec![min.x, min.y, min.z]));
    schematic.put_component("Blocks", blocks_compound);

    let mut root = NbtCompound::new();
    root.put_component("Schematic", schematic);

    // Write gzipped NBT to file
    let file =
        fs::File::create(path).map_err(|e| format!("Failed to create schematic file: {e}"))?;

    write_gzip_compound_tag(root, file).map_err(|e| format!("Failed to write schematic: {e}"))?;

    log::info!(
        "Saved schematic: {width}x{height}x{length} ({} palette entries)",
        next_index
    );

    Ok(())
}
