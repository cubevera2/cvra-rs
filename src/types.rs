use crate::io::vec3_bytes;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Scene {
    pub roots: Vec<Entity>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Script {
    pub source: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpawnLocation;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Entity {
    pub identity: Option<Identity>,
    pub appearance: Option<Appearance>,
    pub script: Option<Script>,
    pub spawn_location: Option<SpawnLocation>,
    pub children: Vec<Entity>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub name: String,
    pub class: String,
    pub locked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color3 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Default for Color3 {
    fn default() -> Self {
        Self {
            r: 255,
            g: 255,
            b: 255,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Angles {
    pub pitch: u16,
    pub yaw: u16,
    pub roll: u16,
}

impl Angles {
    fn encode_degrees(degrees: f32) -> u16 {
        let turns = (degrees / 360.0).rem_euclid(1.0);

        ((turns * 65536.0).round() as i64 & 0xFFFF) as u16
    }

    pub fn from_degrees(pitch: f32, yaw: f32, roll: f32) -> Self {
        Self {
            pitch: Self::encode_degrees(pitch),
            yaw: Self::encode_degrees(yaw),
            roll: Self::encode_degrees(roll),
        }
    }

    pub fn to_degrees(self) -> [f32; 3] {
        [self.pitch, self.yaw, self.roll].map(|v| f32::from(v) / 65536.0 * 360.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    #[default]
    Inherited,
    Hidden,
    Visible,
}

impl Visibility {
    pub(crate) fn bits(self) -> u8 {
        match self {
            Visibility::Inherited => 0,
            Visibility::Hidden => 1,
            Visibility::Visible => 2,
        }
    }

    pub(crate) fn from_bits(bits: u8) -> Option<Self> {
        match bits {
            0 => Some(Visibility::Inherited),
            1 => Some(Visibility::Hidden),
            2 => Some(Visibility::Visible),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Shape {
    Block { size: [f32; 3] },
    Wedge { size: [f32; 3] },
    Cylinder { radius: f32, height: f32 },
    Sphere { radius: f32 },
}

impl Default for Shape {
    fn default() -> Self {
        Shape::Block { size: [1.0; 3] }
    }
}

impl Shape {
    pub(crate) fn write(&self, out: &mut Vec<u8>) {
        match self {
            Shape::Block { size } => {
                out.push(0);

                out.extend_from_slice(&vec3_bytes(*size));
            }

            Shape::Wedge { size } => {
                out.push(1);

                out.extend_from_slice(&vec3_bytes(*size));
            }

            Shape::Cylinder { radius, height } => {
                out.push(2);

                out.extend_from_slice(&radius.to_le_bytes());
                out.extend_from_slice(&height.to_le_bytes());
            }

            Shape::Sphere { radius } => {
                out.push(3);

                out.extend_from_slice(&radius.to_le_bytes());
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Material {
    #[default]
    Plastic,
    Wood,
    Concrete,
    Grass,
    Sand,
    Neon,
}

impl Material {
    pub(crate) fn bits(self) -> u8 {
        match self {
            Material::Plastic => 0,
            Material::Wood => 1,
            Material::Concrete => 2,
            Material::Grass => 3,
            Material::Sand => 4,
            Material::Neon => 7,
        }
    }

    pub(crate) fn from_bits(bits: u8) -> Option<Self> {
        match bits {
            0 => Some(Material::Plastic),
            1 => Some(Material::Wood),
            2 => Some(Material::Concrete),
            3 => Some(Material::Grass),
            4 => Some(Material::Sand),
            7 => Some(Material::Neon),

            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Appearance {
    pub position: [f32; 3],
    pub rotation: Angles,
    pub scale: [f32; 3],
    pub color: Color3,
    pub transparency: u8,
    pub visibility: Visibility,
    pub shape: Shape,
    pub material: Material,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            rotation: Angles::default(),
            scale: [1.0; 3],
            color: Color3::default(),
            transparency: 0,
            visibility: Visibility::default(),
            shape: Shape::default(),
            material: Material::default(),
        }
    }
}
