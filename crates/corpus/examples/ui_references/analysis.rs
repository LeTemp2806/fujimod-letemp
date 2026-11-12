use fujimod_firmware as firmware;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Anchor {
    pub name: String,
    pub value: u32,
}

pub struct Hit {
    pub anchor: String,
    pub kind: &'static str,
    pub offset: usize,
}

pub fn verify_identity(source: &[u8], size: usize, sha256: &str) -> Result<(), String> {
    let actual = firmware::sha256(source);
    if source.len() != size || actual != sha256 {
        return Err(format!(
            "source identity mismatch: size {}, sha256 {actual}",
            source.len()
        ));
    }
    Ok(())
}

/// These are encoding candidates even when Capstone can decode them: stream
/// data can look like A32 instructions. No guessed function boundaries.
pub fn references(bytes: &[u8], base: Option<u64>, anchors: &[Anchor]) -> Vec<Hit> {
    let mut hits = Vec::new();
    for (index, bytes4) in bytes.as_chunks::<4>().0.iter().enumerate() {
        let offset = index * 4;
        let word = u32::from_le_bytes(*bytes4);
        let cond = word >> 28;
        let materialized = bytes.get(offset + 4..offset + 8).and_then(|v| {
            let next = u32::from_le_bytes(v.try_into().unwrap());
            (cond != 15
                && word & 0x0ff0_0000 == 0x0300_0000
                && next & 0x0ff0_0000 == 0x0340_0000
                && word & 0xf000_f000 == next & 0xf000_f000)
                .then_some({
                    ((word >> 4) & 0xf000)
                        | (word & 0xfff)
                        | ((((next >> 4) & 0xf000) | (next & 0xfff)) << 16)
                })
        });
        let branch = if cond != 15 && word & 0x0e00_0000 == 0x0a00_0000 {
            let displacement = i64::from((word << 8) as i32 >> 6);
            base.and_then(|b| (b + offset as u64 + 8).checked_add_signed(displacement))
                .and_then(|a| u32::try_from(a).ok())
        } else {
            None
        };
        let immediate = if cond == 15 {
            None
        } else if word & 0x0ff0_0000 == 0x0300_0000 {
            Some(((word >> 4) & 0xf000) | (word & 0xfff)) // MOVW
        } else if matches!(word & 0x0fe0_0000, 0x03a0_0000 | 0x0340_0000) {
            // MOV or CMP (CMP requires S=1 and Rd=0).
            let is_mov = word & 0x0fe0_0000 == 0x03a0_0000;
            if is_mov || word & 0x0010_f000 == 0x0010_0000 {
                Some((word & 255).rotate_right(((word >> 8) & 15) * 2))
            } else {
                None
            }
        } else {
            None
        };
        let literal = if cond != 15 && word & 0x0f7f_0000 == 0x051f_0000 {
            // LDR Rt,[PC,+/-imm12], P=1,W=0,B=0,L=1; A32 PC is +8.
            let displacement = i64::from(word & 0xfff) * if word & (1 << 23) != 0 { 1 } else { -1 };
            let target = (offset as u64 + 8).checked_add_signed(displacement);
            target
                .and_then(|v| usize::try_from(v).ok())
                .and_then(|at| bytes.get(at..at.checked_add(4)?))
                .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
        } else {
            None
        };
        for anchor in anchors {
            for (kind, matches) in [
                ("word-value", word == anchor.value),
                ("a32-immediate-candidate", immediate == Some(anchor.value)),
                (
                    "a32-literal-load-candidate",
                    base.is_some() && literal == Some(anchor.value),
                ),
                (
                    "a32-address-pair-candidate",
                    materialized == Some(anchor.value),
                ),
                ("a32-direct-branch-candidate", branch == Some(anchor.value)),
            ] {
                if matches {
                    hits.push(Hit {
                        anchor: anchor.name.clone(),
                        kind,
                        offset,
                    });
                }
            }
        }
    }
    hits
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scan(words: &[u32]) -> Vec<Hit> {
        let bytes: Vec<_> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        references(
            &bytes,
            Some(0x1000),
            &[Anchor {
                name: "3d".into(),
                value: 1348,
            }],
        )
    }
    #[test]
    fn exact_identity_required() {
        let hash = firmware::sha256(b"official");
        assert!(verify_identity(b"official", 8, &hash).is_ok());
        assert!(verify_identity(b"officiaL", 8, &hash).is_err());
        assert!(verify_identity(b"official", 7, &hash).is_err());
    }
    #[test]
    fn movw_literal_and_data_are_distinct() {
        let hits = scan(&[0xe300_1544, 0xe59f_0000, 0xe1a0_0000, 1348]);
        assert_eq!(hits.len(), 3);
        assert_eq!(
            (hits[0].offset, hits[0].kind),
            (0, "a32-immediate-candidate")
        );
        assert_eq!(
            (hits[1].offset, hits[1].kind),
            (4, "a32-literal-load-candidate")
        );
        assert_eq!((hits[2].offset, hits[2].kind), (12, "word-value"));
    }
    #[test]
    fn negative_literal_and_bounds() {
        let hits = scan(&[1348, 0xe51f_000c, 0xe59f_0fff, 0xe51f_0fff]);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[1].offset, 4);
    }
    #[test]
    fn excludes_movt_unconditional_and_byte_loads() {
        assert!(scan(&[0xe340_1544, 0xf300_1544, 0xe5df_0544]).is_empty());
    }
    #[test]
    fn unmapped_units_do_not_invent_literal_addresses() {
        let bytes: Vec<_> = [0xe59f_0000_u32, 0, 1348]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let hits = references(
            &bytes,
            None,
            &[Anchor {
                name: "3d".into(),
                value: 1348,
            }],
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].kind, "word-value");
    }

    #[test]
    fn materializations_require_matching_condition_and_register() {
        let bytes: Vec<_> = [
            0xe305_0678_u32,
            0xe341_0234,
            0xe305_0678,
            0xe341_1234,
            0x0305_0678,
            0xe341_0234,
        ]
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .collect();
        let hits = references(
            &bytes,
            Some(0x1000),
            &[Anchor {
                name: "pointer".into(),
                value: 0x12345678,
            }],
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].kind, "a32-address-pair-candidate");
    }

    #[test]
    fn forward_and_backward_branches_use_signed_pc_displacement() {
        let bytes: Vec<_> = [0xeb00_0000_u32, 0xeaff_fffd, 0xfaff_fffc]
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .collect();
        let hits = references(
            &bytes,
            Some(0x1000),
            &[
                Anchor {
                    name: "target".into(),
                    value: 0x1000,
                },
                Anchor {
                    name: "next".into(),
                    value: 0x1008,
                },
            ],
        );
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].anchor, "next");
        assert_eq!(hits[1].anchor, "target");
    }

    #[test]
    fn rotated_immediates_include_mov_and_valid_cmp_only() {
        let bytes: Vec<_> = [0xe3a0_0c01_u32, 0xe350_0c01, 0xe340_0c01, 0xe350_1c01]
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .collect();
        let hits = references(
            &bytes,
            Some(0x1000),
            &[Anchor {
                name: "scene".into(),
                value: 256,
            }],
        );
        assert_eq!(hits.len(), 2);
        assert_eq!((hits[0].offset, hits[1].offset), (0, 4));
    }
}
