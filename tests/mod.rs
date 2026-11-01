use cvra_rs::{
    Angles, Appearance, Color3, DecodeLimits, Entity, Error, Identity, Scene, Shape, Visibility,
};

const SPEC_EXAMPLE: &[u8] = &[
    0x43, 0x56, 0x52, 0x41, 0x01, 0x00, 0x00, 0x02, 0x06, b'G', b'r', b'o', b'u', b'n', b'd', 0x04,
    b'P', b'a', b'r', b't', 0x01, 0x03, 0x02, 0x00, 0x01, 0x1D, 0x45, 0x00, 0x00, 0x00, 0x00, 0xCD,
    0xCC, 0xCC, 0xBD, 0x00, 0x00, 0x00, 0x00, 0x4D, 0x99, 0x4D, 0x00, 0x00, 0x00, 0x48, 0x42, 0xCD,
    0xCC, 0x4C, 0x3E, 0x00, 0x00, 0x48, 0x42, 0x00,
];

fn ground() -> Scene {
    Scene {
        roots: vec![Entity {
            identity: Some(Identity {
                name: "Ground".into(),
                class: "Part".into(),
            }),
            appearance: Some(Appearance {
                position: [0.0, -0.1, 0.0],
                color: Color3 {
                    r: 77,
                    g: 153,
                    b: 77,
                },
                shape: Shape::Block {
                    size: [50.0, 0.2, 50.0],
                },
                ..Appearance::default()
            }),
            children: vec![],
        }],
    }
}

#[test]
fn decodes_the_spec_example() {
    assert_eq!(cvra_rs::decode(SPEC_EXAMPLE).unwrap(), ground());
}

#[test]
fn encodes_the_spec_example_exactly() {
    assert_eq!(cvra_rs::encode(&ground()).unwrap(), SPEC_EXAMPLE);
}

#[test]
fn default_appearance_is_a_one_byte_payload() {
    let scene = Scene {
        roots: vec![Entity {
            appearance: Some(Appearance::default()),
            ..Entity::default()
        }],
    };

    let bytes = cvra_rs::encode(&scene).unwrap();

    assert_eq!(bytes.len(), 13);
    assert_eq!(cvra_rs::decode(&bytes).unwrap(), scene);
}

#[test]
fn round_trips_a_tree() {
    let part = |name: &str, x: f32| Entity {
        identity: Some(Identity {
            name: name.into(),
            class: "Part".into(),
        }),
        appearance: Some(Appearance {
            position: [x, 1.0, -0.0],
            rotation: Angles::from_degrees(10.0, 90.0, 350.0),
            transparency: 128,
            visibility: Visibility::Hidden,
            ..Appearance::default()
        }),
        children: vec![],
    };

    let scene = Scene {
        roots: vec![Entity {
            identity: Some(Identity {
                name: "Props".into(),
                class: "Model".into(),
            }),
            appearance: None,
            children: vec![part("A", 1.0), part("B", 2.0)],
        }],
    };

    let bytes = cvra_rs::encode(&scene).unwrap();

    assert_eq!(cvra_rs::decode(&bytes).unwrap(), scene);
}

#[cfg(feature = "zstd")]
#[test]
fn round_trips_compressed() {
    let options = cvra_rs::EncodeOptions {
        compress: true,
        ..Default::default()
    };

    let bytes = cvra_rs::encode_with(&ground(), &options).unwrap();

    assert_eq!(bytes[6], 1);
    assert_eq!(cvra_rs::decode(&bytes).unwrap(), ground());
}

#[test]
fn rejects_bad_signature() {
    let mut bytes = SPEC_EXAMPLE.to_vec();

    bytes[0] = b'X';

    assert!(matches!(cvra_rs::decode(&bytes), Err(Error::BadSignature)));
}

#[test]
fn rejects_newer_versions() {
    let mut bytes = SPEC_EXAMPLE.to_vec();

    bytes[4] = 2;

    assert!(matches!(
        cvra_rs::decode(&bytes),
        Err(Error::UnsupportedVersion(2))
    ));
}

#[test]
fn rejects_reserved_file_flags() {
    let mut bytes = SPEC_EXAMPLE.to_vec();

    bytes[6] = 0b10;

    assert!(matches!(
        cvra_rs::decode(&bytes),
        Err(Error::ReservedBits(_))
    ));
}

#[test]
fn rejects_bad_visibility() {
    let mut bytes = SPEC_EXAMPLE.to_vec();

    bytes[26] |= 0b11 << 4;

    assert!(matches!(
        cvra_rs::decode(&bytes),
        Err(Error::InvalidVisibility)
    ));
}

#[test]
fn round_trips_scale() {
    let scene = Scene {
        roots: vec![Entity {
            appearance: Some(Appearance {
                scale: [2.0, 0.5, -1.0],
                ..Appearance::default()
            }),
            ..Entity::default()
        }],
    };

    let bytes = cvra_rs::encode(&scene).unwrap();

    assert_eq!(cvra_rs::decode(&bytes).unwrap(), scene);

    let plain = Scene {
        roots: vec![Entity {
            appearance: Some(Appearance::default()),
            ..Entity::default()
        }],
    };

    assert_eq!(cvra_rs::encode(&plain).unwrap().len(), 13);
}

#[test]
fn skips_unknown_components() {
    let bytes = [
        0x43, 0x56, 0x52, 0x41, 0x01, 0x00, 0x00, 0x00, 0x01, 0x20, 0x03, 0xAA, 0xBB, 0xCC, 0x00,
    ];

    let scene = cvra_rs::decode(&bytes).unwrap();

    assert_eq!(scene.roots, vec![Entity::default()]);
}

#[test]
fn rejects_truncated_input() {
    for len in 0..SPEC_EXAMPLE.len() {
        assert!(cvra_rs::decode(&SPEC_EXAMPLE[..len]).is_err(), "len {len}");
    }
}

#[test]
fn enforces_depth_limit() {
    let mut entity = Entity::default();

    for _ in 0..10 {
        entity = Entity {
            children: vec![entity],
            ..Entity::default()
        };
    }

    let bytes = cvra_rs::encode(&Scene {
        roots: vec![entity],
    })
    .unwrap();

    let limits = DecodeLimits {
        max_depth: 5,
        ..DecodeLimits::default()
    };

    assert!(matches!(
        cvra_rs::decode_with(&bytes, &limits),
        Err(Error::LimitExceeded(_))
    ));
}

#[test]
fn varint_edge_cases() {
    let name = "a".repeat(128);
    let scene = Scene {
        roots: vec![Entity {
            identity: Some(Identity {
                name,
                class: String::new(),
            }),
            ..Entity::default()
        }],
    };

    let bytes = cvra_rs::encode(&scene).unwrap();

    assert_eq!(cvra_rs::decode(&bytes).unwrap(), scene);
}
