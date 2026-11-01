use crate::{
    COMPONENT_APPEARANCE, COMPONENT_IDENTITY, COMPONENT_LOCKED, COMPONENT_MATERIAL,
    COMPONENT_SCRIPT, COMPONENT_SPAWN_LOCATION, FLAG_COMPRESSED, SIGNATURE, VERSION,
    error::{Error, Result},
    io::{put_len, put_string, put_varint, vec3_bytes},
    types::{Appearance, Entity, Material, Scene},
};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct EncodeOptions {
    pub compress: bool,
    pub compression_level: i32,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        Self {
            compress: false,
            compression_level: 3,
        }
    }
}

pub fn encode(scene: &Scene) -> Result<Vec<u8>> {
    encode_with(scene, &EncodeOptions::default())
}

pub fn encode_with(scene: &Scene, options: &EncodeOptions) -> Result<Vec<u8>> {
    let mut table = StringTable::default();
    let mut entities = Vec::new();

    for root in &scene.roots {
        encode_entity(root, &mut table, &mut entities)?;
    }

    let mut body = Vec::new();

    put_len(&mut body, table.list.len())?;

    for string in &table.list {
        put_string(&mut body, string)?;
    }

    put_len(&mut body, scene.roots.len())?;
    body.extend_from_slice(&entities);

    let (flags, body) = if options.compress {
        (FLAG_COMPRESSED, compress(&body, options.compression_level)?)
    } else {
        (0, body)
    };

    let mut out = Vec::with_capacity(7 + body.len());

    out.extend_from_slice(&SIGNATURE);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.push(flags);
    out.extend_from_slice(&body);

    Ok(out)
}

#[cfg(feature = "zstd")]
fn compress(body: &[u8], level: i32) -> Result<Vec<u8>> {
    zstd::stream::encode_all(body, level).map_err(Error::Compression)
}

#[cfg(not(feature = "zstd"))]
fn compress(_body: &[u8], _level: i32) -> Result<Vec<u8>> {
    Err(Error::CompressionUnavailable)
}

#[derive(Default)]
struct StringTable {
    map: HashMap<String, u32>,
    list: Vec<String>,
}

impl StringTable {
    fn intern(&mut self, s: &str) -> Result<u32> {
        if let Some(&index) = self.map.get(s) {
            return Ok(index);
        }

        let index = u32::try_from(self.list.len()).map_err(|_| Error::TooLarge)?;

        self.list.push(s.to_owned());
        self.map.insert(s.to_owned(), index);

        Ok(index)
    }
}

fn encode_entity(entity: &Entity, table: &mut StringTable, out: &mut Vec<u8>) -> Result<()> {
    let mut mask: u32 = 0;
    let mut records = Vec::new();

    if let Some(identity) = &entity.identity {
        mask |= 1 << COMPONENT_IDENTITY;

        let name = table.intern(&identity.name)?;
        let class = table.intern(&identity.class)?;

        let mut payload = Vec::new();

        put_varint(&mut payload, name);
        put_varint(&mut payload, class);

        put_len(&mut records, payload.len())?;

        records.extend_from_slice(&payload);
    }

    if let Some(appearance) = &entity.appearance {
        mask |= 1 << COMPONENT_APPEARANCE;

        let payload = encode_appearance(appearance);

        put_len(&mut records, payload.len())?;

        records.extend_from_slice(&payload);
    }

    if let Some(script) = &entity.script {
        mask |= 1 << COMPONENT_SCRIPT;

        let payload = script.source.as_bytes();

        put_len(&mut records, payload.len())?;

        records.extend_from_slice(payload);
    }

    if entity.spawn_location.is_some() {
        mask |= 1 << COMPONENT_SPAWN_LOCATION;

        put_len(&mut records, 0)?;
    }

    if entity.identity.as_ref().is_some_and(|i| i.locked) {
        mask |= 1 << COMPONENT_LOCKED;

        put_len(&mut records, 0)?;
    }

    if let Some(appearance) = &entity.appearance
        && appearance.material != Material::default()
    {
        mask |= 1 << COMPONENT_MATERIAL;

        put_len(&mut records, 1)?;

        records.push(appearance.material.bits());
    }

    put_varint(out, mask);

    out.extend_from_slice(&records);

    put_len(out, entity.children.len())?;

    for child in &entity.children {
        encode_entity(child, table, out)?;
    }

    Ok(())
}

fn encode_appearance(a: &Appearance) -> Vec<u8> {
    let default = Appearance::default();

    let mut flags: u8 = 0;
    let mut body = Vec::new();

    let position = vec3_bytes(a.position);

    if position != vec3_bytes(default.position) {
        flags |= 1 << 0;

        body.extend_from_slice(&position);
    }

    if a.rotation != default.rotation {
        flags |= 1 << 1;

        body.extend_from_slice(&a.rotation.pitch.to_le_bytes());
        body.extend_from_slice(&a.rotation.yaw.to_le_bytes());
        body.extend_from_slice(&a.rotation.roll.to_le_bytes());
    }

    if a.color != default.color {
        flags |= 1 << 2;

        body.extend_from_slice(&[a.color.r, a.color.g, a.color.b]);
    }

    if a.transparency != default.transparency {
        flags |= 1 << 3;

        body.push(a.transparency);
    }

    flags |= a.visibility.bits() << 4;

    let mut shape = Vec::new();
    let mut default_shape = Vec::new();

    a.shape.write(&mut shape);

    default.shape.write(&mut default_shape);

    if shape != default_shape {
        flags |= 1 << 6;

        body.extend_from_slice(&shape);
    }

    let scale = vec3_bytes(a.scale);

    if scale != vec3_bytes(default.scale) {
        flags |= 1 << 7;

        body.extend_from_slice(&scale);
    }

    let mut payload = Vec::with_capacity(1 + body.len());

    payload.push(flags);
    payload.extend_from_slice(&body);

    payload
}
