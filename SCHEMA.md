# .cvra format

all multi-byte values are little-endian. strings are utf-8

## primitive types

| type   | size   | description                                                    |
| ------ | ------ | -------------------------------------------------------------- |
| u8     | 1      | unsigned byte                                                  |
| u16    | 2      | unsigned 16-bit integer                                        |
| f32    | 4      | ieee 754 single-precision float                                |
| varint | 1-5    | unsigned leb128 (7 bits per byte, high bit = continue)         |
| string | varies | `varint` byte length, then utf-8 bytes (no terminator)         |
| vec3   | 12     | three `f32`: x, y, z                                           |
| color3 | 3      | three `u8`: r, g, b (0-255)                                    |
| alpha  | 1      | `u8` transparency: 0 = opaque, 255 = fully transparent         |
| angles | 6      | three `u16`: pitch, yaw, roll. `degrees = value / 65536 * 360` |

- rotation uses euler angles applied in yxz order
- position and rotation are always stored as separate fields

## file structure

| field     | type    | description                                       |
| --------- | ------- | ------------------------------------------------- |
| signature | 4 bytes | always `CVRA` (or `67 86 82 65`)                  |
| version   | u16     | the format version (currently `1`)                |
| flags     | u8      | bit 0: body is zstd-compressed. bits 1-7 reserved |
| body      | bytes   | everything below (compressed if flag 0 is set)    |

### body

| field        | type        | description                                 |
| ------------ | ----------- | ------------------------------------------- |
| string table | StringTable | all names and class names, interned         |
| root count   | varint      | the number of root entities in the scene    |
| roots        | Entity[]    | root entities, each followed by its subtree |

### string table

| field   | type     | description                   |
| ------- | -------- | ----------------------------- |
| count   | varint   | number of strings             |
| strings | string[] | referenced elsewhere by index |

- any string field in a component (names, classes) is stored as a `varint` index into this table
- repeated values like `"Part"` cost one byte each

## entity

- entities are stored depth-first
- there is no parent id
- an entity's children immediately follow it, which replaces `ChildOf` in the file

| field          | type        | description                                |
| -------------- | ----------- | ------------------------------------------ |
| component mask | varint      | bit n set = component with id n is present |
| components     | Component[] | one per set bit, in ascending id order     |
| child count    | varint      | number of direct children                  |
| children       | Entity[]    | each child entity, recursively             |

### component record

| field   | type   | description                                 |
| ------- | ------ | ------------------------------------------- |
| length  | varint | payload size in bytes                       |
| payload | bytes  | component data, layout depends on component |

the length prefix lets readers skip components they don't know, so new components can be added without bumping the version

## component ids

| id  | component     |
| --- | ------------- |
| 0   | Identity      |
| 1   | Appearance    |
| 2   | Script        |
| 3   | SpawnLocation |
| 4   | Locked        |
| 5+  | reserved      |

### identity (id 0)

| field | type   | description        |
| ----- | ------ | ------------------ |
| name  | varint | string table index |
| class | varint | string table index |

### appearance (id 1)

- fields are optional and gated by a flag byte
- absent fields take their default, so a plain white 1x1x1 block at the origin is a 2-byte payload

| field        | type   | description                                     |
| ------------ | ------ | ----------------------------------------------- |
| flags        | u8     | see below                                       |
| position     | vec3   | only if bit 0 is set. default `(0, 0, 0)`       |
| rotation     | angles | only if bit 1 is set. default `(0, 0, 0)`       |
| color        | color3 | only if bit 2 is set. default `(255, 255, 255)` |
| transparency | alpha  | only if bit 3 is set. default `0`               |
| shape        | Shape  | only if bit 6 is set. default `Block(1, 1, 1)`  |

flags:

| bits | meaning                                                          |
| ---- | ---------------------------------------------------------------- |
| 0    | has position                                                     |
| 1    | has rotation                                                     |
| 2    | has color                                                        |
| 3    | has transparency                                                 |
| 4-5  | visibility: `0` inherited, `1` hidden, `2` visible (`3` invalid) |
| 6    | has shape                                                        |
| 7    | reserved, must be 0                                              |

- visibility lives in the flags byte, since inherited is the default and a 2-bit enum costs no extra payload

### script (id 2)

- the payload is the script source as raw utf-8, with no length prefix of its own (the component record's length is the source length)

### spawn location (id 3)

- a marker component with an empty payload (so it has a length of `0`)
- an entity that has it is a place where players can spawn
- it is normally combined with identity and appearance so the spawn has a position and a visible pad
- a spawn location entity with only identity, appearance and this marker has mask `0B`, and its spawn record is a single `00` byte

### locked (id 4)

- a marker component with an empty payload (length `0`)
- it marks the entity's identity as locked, meaning the user shouldn't be able to select the entity by clicking in the viewport
- it only has meaning on an entity that also has identity, and readers ignore it otherwise

### shape

| field | type | description    |
| ----- | ---- | -------------- |
| tag   | u8   | shape variant  |
| data  | ...  | depends on tag |

| tag | variant  | data         |
| --- | -------- | ------------ |
| 0   | block    | `size: vec3` |
| 1+  | reserved | (tbd)        |

## example

note: this is probably outdated :pensive:

```
43 56 52 41                           signature "CVRA"
01 00                                 version 1
00                                    flags (uncompressed)

02                                    string table: 2 strings
06 47 72 6F 75 6E 64                  [0] "Ground"
04 50 61 72 74                        [1] "Part"

01                                    1 root entity

03                                    mask: identity + appearance
02 00 01                              identity: len 2, name=0, class=1
1D                                    appearance: len 29
45                                    flags: position | color | shape
00 00 00 00 CD CC CC BD 00 00 00 00   position (0, -0.1, 0)
4D 99 4D                              color (77, 153, 77)
00                                    shape: block
00 00 48 42 CD CC 4C 3E 00 00 48 42   size (50, 0.2, 50)
00                                    child count: 0
```

## reader rules

- reject files whose signature doesn't match or whose version is newer than supported
- skip unknown component ids using their length prefix
- reject reserved bits that are set (flags byte, appearance flags)
- entity and string table limits are up to the loader (suggest capping depth and count to avoid abuse)
