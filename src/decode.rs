use crate::{
    COMPONENT_APPEARANCE, COMPONENT_IDENTITY, COMPONENT_LOCKED, COMPONENT_MATERIAL,
    COMPONENT_SCRIPT, COMPONENT_SPAWN_LOCATION, FLAG_COMPRESSED, SIGNATURE, Script, VERSION,
    error::{Error, Result},
    io::Reader,
    types::{
        Angles, Appearance, Color3, Entity, Identity, Material, Scene, Shape, SpawnLocation,
        Visibility,
    },
};

#[derive(Debug, Clone, Copy)]
pub struct DecodeLimits {
    pub max_depth: usize,
    pub max_entities: usize,
    pub max_strings: usize,
    pub max_string_len: usize,
    pub max_body_size: usize,
}

impl Default for DecodeLimits {
    fn default() -> Self {
        Self {
            max_depth: 256,
            max_entities: 1_000_000,
            max_strings: 1_000_000,
            max_string_len: 64 * 1024,
            max_body_size: 256 * 1024 * 1024,
        }
    }
}

pub fn decode(bytes: &[u8]) -> Result<Scene> {
    decode_with(bytes, &DecodeLimits::default())
}

pub fn decode_with(bytes: &[u8], limits: &DecodeLimits) -> Result<Scene> {
    let mut header = Reader::new(bytes);

    if header.bytes(4)? != &SIGNATURE[..] {
        return Err(Error::BadSignature);
    }

    let version = header.u16()?;

    if version > VERSION {
        return Err(Error::UnsupportedVersion(version));
    }

    let flags = header.u8()?;

    if flags & !FLAG_COMPRESSED != 0 {
        return Err(Error::ReservedBits("file flags"));
    }

    let rest = header.rest();

    let decompressed;
    let body: &[u8] = if flags & FLAG_COMPRESSED != 0 {
        decompressed = decompress(rest, limits.max_body_size)?;

        &decompressed
    } else {
        if rest.len() > limits.max_body_size {
            return Err(Error::LimitExceeded("body size"));
        }

        rest
    };

    decode_body(body, limits)
}

#[cfg(feature = "zstd")]
fn decompress(data: &[u8], max: usize) -> Result<Vec<u8>> {
    use std::io::Read;

    let decoder = zstd::stream::read::Decoder::new(data).map_err(Error::Compression)?;
    let mut out = Vec::new();

    decoder
        .take(max as u64 + 1)
        .read_to_end(&mut out)
        .map_err(Error::Compression)?;

    if out.len() > max {
        return Err(Error::LimitExceeded("decompressed body size"));
    }

    Ok(out)
}

#[cfg(not(feature = "zstd"))]
fn decompress(_data: &[u8], _max: usize) -> Result<Vec<u8>> {
    Err(Error::CompressionUnavailable)
}

struct Context<'a> {
    strings: Vec<String>,
    limits: &'a DecodeLimits,
    entities: usize,
}

fn decode_body(body: &[u8], limits: &DecodeLimits) -> Result<Scene> {
    let mut r = Reader::new(body);

    let string_count = r.varint()? as usize;

    if string_count > limits.max_strings {
        return Err(Error::LimitExceeded("string table size"));
    }

    let mut strings = Vec::with_capacity(string_count.min(r.remaining()));

    for _ in 0..string_count {
        let len = r.varint()? as usize;

        if len > limits.max_string_len {
            return Err(Error::LimitExceeded("string length"));
        }

        let text = std::str::from_utf8(r.bytes(len)?).map_err(|_| Error::InvalidUtf8)?;

        strings.push(text.to_owned());
    }

    let root_count = r.varint()? as usize;

    let mut ctx = Context {
        strings,
        limits,
        entities: 0,
    };

    let mut roots = Vec::with_capacity(root_count.min(r.remaining()));

    for _ in 0..root_count {
        roots.push(read_entity(&mut r, &mut ctx, 0)?);
    }

    if !r.is_empty() {
        return Err(Error::TrailingData("body"));
    }

    Ok(Scene { roots })
}

fn read_script(r: &mut Reader<'_>) -> Result<Script> {
    let len = r.remaining();
    let source = std::str::from_utf8(r.bytes(len)?)
        .map_err(|_| Error::InvalidUtf8)?
        .to_owned();

    Ok(Script { source })
}

fn read_material(r: &mut Reader<'_>) -> Result<Material> {
    let tag = r.u8()?;

    Material::from_bits(tag).ok_or(Error::UnknownMaterial(tag))
}

fn read_entity(r: &mut Reader<'_>, ctx: &mut Context<'_>, depth: usize) -> Result<Entity> {
    if depth > ctx.limits.max_depth {
        return Err(Error::LimitExceeded("entity depth"));
    }

    ctx.entities += 1;

    if ctx.entities > ctx.limits.max_entities {
        return Err(Error::LimitExceeded("entity count"));
    }

    let mask = r.varint()?;

    let mut entity = Entity::default();

    for id in 0..32u32 {
        if mask & (1 << id) == 0 {
            continue;
        }

        let len = r.varint()? as usize;

        let mut payload = Reader::new(r.bytes(len)?);

        match id {
            COMPONENT_IDENTITY => entity.identity = Some(read_identity(&mut payload, ctx)?),
            COMPONENT_APPEARANCE => entity.appearance = Some(read_appearance(&mut payload)?),
            COMPONENT_SCRIPT => entity.script = Some(read_script(&mut payload)?),
            COMPONENT_SPAWN_LOCATION => entity.spawn_location = Some(SpawnLocation),

            COMPONENT_LOCKED => {
                if let Some(identity) = entity.identity.as_mut() {
                    identity.locked = true;
                }
            }

            COMPONENT_MATERIAL => {
                let material = read_material(&mut payload)?;

                if let Some(appearance) = entity.appearance.as_mut() {
                    appearance.material = material;
                }
            }

            _ => continue,
        }

        if !payload.is_empty() {
            return Err(Error::TrailingData("component"));
        }
    }

    let child_count = r.varint()? as usize;

    entity.children = Vec::with_capacity(child_count.min(r.remaining()));

    for _ in 0..child_count {
        entity.children.push(read_entity(r, ctx, depth + 1)?);
    }

    Ok(entity)
}

fn string(ctx: &Context<'_>, index: u32) -> Result<String> {
    ctx.strings
        .get(index as usize)
        .cloned()
        .ok_or(Error::InvalidStringIndex(index))
}

fn read_identity(r: &mut Reader<'_>, ctx: &Context<'_>) -> Result<Identity> {
    let name = string(ctx, r.varint()?)?;
    let class = string(ctx, r.varint()?)?;

    Ok(Identity {
        name,
        class,
        locked: false,
    })
}

fn read_appearance(r: &mut Reader<'_>) -> Result<Appearance> {
    let flags = r.u8()?;
    let visibility = Visibility::from_bits((flags >> 4) & 0b11).ok_or(Error::InvalidVisibility)?;

    let mut appearance = Appearance {
        visibility,

        ..Appearance::default()
    };

    if flags & (1 << 0) != 0 {
        appearance.position = r.vec3()?;
    }

    if flags & (1 << 1) != 0 {
        appearance.rotation = Angles {
            pitch: r.u16()?,
            yaw: r.u16()?,
            roll: r.u16()?,
        };
    }

    if flags & (1 << 2) != 0 {
        appearance.color = Color3 {
            r: r.u8()?,
            g: r.u8()?,
            b: r.u8()?,
        };
    }

    if flags & (1 << 3) != 0 {
        appearance.transparency = r.u8()?;
    }

    if flags & (1 << 6) != 0 {
        appearance.shape = read_shape(r)?;
    }

    if flags & (1 << 7) != 0 {
        appearance.scale = r.vec3()?;
    }

    Ok(appearance)
}

fn read_shape(r: &mut Reader<'_>) -> Result<Shape> {
    match r.u8()? {
        0 => Ok(Shape::Block { size: r.vec3()? }),
        1 => Ok(Shape::Wedge { size: r.vec3()? }),
        2 => Ok(Shape::Cylinder {
            radius: r.f32()?,
            height: r.f32()?,
        }),
        3 => Ok(Shape::Sphere { radius: r.f32()? }),

        tag => Err(Error::UnknownShape(tag)),
    }
}
