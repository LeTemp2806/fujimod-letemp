//! Read-only candidate inventory, not a function locator or patch builder.
//! cargo run -p fujimod-corpus --example ui_references -- SPEC DAT NEW_DIRECTORY
#[path = "ui_references/analysis.rs"]
mod analysis;

use capstone::prelude::*;
use fujimod_firmware as firmware;
use serde::Deserialize;
use serde_json::json;
use std::{error::Error, fs, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Spec {
    firmware_id: String,
    size: usize,
    sha256: String,
    version_major: u32,
    version_minor: u32,
    anchors: Vec<analysis::Anchor>,
    #[serde(default)]
    windows: Vec<Window>,
    #[serde(default)]
    strings: Vec<StringProbe>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StringProbe {
    address: u64,
    value: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Window {
    name: String,
    dat_offset: usize,
    offset: usize,
    size: usize,
}

fn instructions(
    cs: &Capstone,
    bytes: &[u8],
    base: Option<u64>,
    start: usize,
    end: usize,
) -> Vec<serde_json::Value> {
    (start..end)
        .step_by(4)
        .map(|offset| {
            let address = base.unwrap_or(0) + offset as u64;
            let decoded = cs.disasm_all(&bytes[offset..offset + 4], address).ok();
            let text = decoded.as_ref().and_then(|v| v.iter().next()).map(|i| {
                format!(
                    "{} {}",
                    i.mnemonic().unwrap_or(""),
                    i.op_str().unwrap_or("")
                )
            });
            json!({"offset":offset,"address":base.map(|b|b+offset as u64),
            "instruction":text})
        })
        .collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    run(&args)
}

fn run(args: &[std::ffi::OsString]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3 {
        return Err("expected SPEC DAT NEW_DIRECTORY; no files are overwritten".into());
    }
    let spec_bytes = fs::read(&args[0])?;
    let spec: Spec = serde_json::from_slice(&spec_bytes)?;
    let source = fs::read(&args[1])?;
    analysis::verify_identity(&source, spec.size, &spec.sha256)?;
    let outer = firmware::outer_header(&source)?;
    if (outer.version_major, outer.version_minor) != (spec.version_major, spec.version_minor) {
        return Err(format!(
            "outer version words {:#x}/{:#x} do not match specification {:#x}/{:#x}",
            outer.version_major, outer.version_minor, spec.version_major, spec.version_minor
        )
        .into());
    }
    let candidates = firmware::scan(&source);
    if candidates.is_empty() {
        return Err("no validated method-4 streams; no report emitted".into());
    }
    let mut strings = Vec::new();
    for probe in &spec.strings {
        let matches: Vec<_> = candidates.iter().filter_map(|c| {
            let header = c.header.as_ref().filter(|h|h.word_bits==32)?;
            let offset = usize::try_from(probe.address.checked_sub(header.base)?).ok()?;
            let end = offset.checked_add(probe.value.len())?;
            (c.decoded.get(offset..end)==Some(probe.value.as_bytes()) && c.decoded.get(end)==Some(&0))
                .then_some(json!({"address":probe.address,"value":probe.value,"dat_offset":c.offset,"offset":offset}))
        }).collect();
        if matches.len() != 1 {
            return Err(format!(
                "string probe {} has {} matches; expected exactly one",
                probe.value,
                matches.len()
            )
            .into());
        }
        strings.push(matches[0].clone());
    }
    for window in &spec.windows {
        let candidate = candidates
            .iter()
            .find(|c| c.offset == window.dat_offset)
            .ok_or("window stream not found")?;
        if window.offset % 4 != 0
            || window.size == 0
            || window.size % 4 != 0
            || window
                .offset
                .checked_add(window.size)
                .is_none_or(|end| end > candidate.decoded.len())
        {
            return Err(format!("invalid window {}", window.name).into());
        }
    }
    // create_dir refuses an existing directory, including the input's directory.
    let output = Path::new(&args[2]);
    fs::create_dir(output)?;
    let cs = Capstone::new()
        .arm()
        .mode(arch::arm::ArchMode::Arm)
        .build()?;
    let mut streams = Vec::new();
    for candidate in candidates {
        let name = format!("stream-{:08x}.bin", candidate.offset);
        fs::write(output.join(&name), &candidate.decoded)?;
        let header = candidate.header.as_ref();
        let base = header.filter(|h| h.word_bits == 32).map(|h| h.base);
        let hits = analysis::references(&candidate.decoded, base, &spec.anchors);
        let windows: Vec<_> = spec
            .windows
            .iter()
            .filter(|w| w.dat_offset == candidate.offset)
            .map(|w| {
                json!({"name":w.name,"offset":w.offset,"size":w.size,
                "sha256":firmware::sha256(&candidate.decoded[w.offset..w.offset+w.size]),
                "instructions":instructions(&cs,&candidate.decoded,base,w.offset,w.offset+w.size)})
            })
            .collect();
        let hits: Vec<_> = hits
            .into_iter()
            .map(|hit| {
                let start = hit.offset.saturating_sub(16) & !3;
                let end = (hit.offset + 24).min(candidate.decoded.len()) & !3;
                // Decode one word at a time: invalid words must not hide later hits.
                let context = instructions(&cs, &candidate.decoded, base, start, end);
                json!({"anchor":hit.anchor,"kind":hit.kind,"offset":hit.offset,
                "address":base.map(|b|b+hit.offset as u64),"context":context})
            })
            .collect();
        streams.push(json!({
            "dat_offset":candidate.offset,"packed_size":candidate.compression.packed_size,
            "decoded_size":candidate.decoded.len(),"sha256":firmware::sha256(&candidate.decoded),
            "path":name,"header":header.map(|h|json!({"word_bits":h.word_bits,
                "base":h.base,"initialized_size":h.initialized_size,"entry":h.entry})),
            "hits":hits,"windows":windows
        }));
    }
    let report = json!({"schema":1,"firmware_id":spec.firmware_id,
        "spec_sha256":firmware::sha256(&spec_bytes),"strings":strings,
        "source":{"size":source.len(),"sha256":firmware::sha256(&source)},
        "outer":{"kind":outer.kind,"version_major":outer.version_major,"version_minor":outer.version_minor},
        "limitations":["Numeric matches are candidates, not semantic references or function boundaries.",
            "Addresses come from stream headers, not verified runtime mappings; overlays may overlap.",
            "Aligned A32 MOV/MOVW/CMP immediates, adjacent MOVW/MOVT, direct branches and PC-relative word loads are scanned.",
            "Thumb, computed IDs, cross-stream loads and control/data reachability are not resolved.",
            "No firmware transition executed, patch applied, signature verified or recovery demonstrated."],
        "streams":streams});
    fs::write(
        output.join("references.generated.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    // Compact, reproducible evidence without redistributing firmware bytes.
    let evidence_streams: Vec<_> = report["streams"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            let mut counts = std::collections::BTreeMap::<String, usize>::new();
            for hit in s["hits"].as_array().unwrap() {
                *counts
                    .entry(format!(
                        "{}:{}",
                        hit["anchor"].as_str().unwrap(),
                        hit["kind"].as_str().unwrap()
                    ))
                    .or_default() += 1;
            }
            json!({"dat_offset":s["dat_offset"],"packed_size":s["packed_size"],
            "decoded_size":s["decoded_size"],"sha256":s["sha256"],"header":s["header"],
            "hit_counts":counts,"windows":s["windows"]})
        })
        .collect();
    let evidence = json!({"schema":1,"firmware_id":report["firmware_id"],
        "spec_sha256":report["spec_sha256"],"strings":report["strings"],
        "source":report["source"],"outer":report["outer"],"limitations":report["limitations"],
        "streams":evidence_streams});
    fs::write(
        output.join("evidence.generated.json"),
        serde_json::to_vec_pretty(&evidence)?,
    )?;
    println!("{}", output.join("references.generated.json").display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);

    struct Fixture(std::path::PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "fujimod-ui-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn arguments(
            &self,
            version_minor: u32,
            windows: serde_json::Value,
            strings: serde_json::Value,
        ) -> Vec<std::ffi::OsString> {
            // A synthetic type-6 container with one literal-only method-4 unit.
            let mut decoded = vec![0_u8; 64];
            for (index, value) in [0_u32, 0x1000, 64, 0x1020, 0, 0, 0, 0xaaaaffff]
                .iter()
                .enumerate()
            {
                decoded[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
            }
            decoded[32..36].copy_from_slice(&0xe300_0544_u32.to_le_bytes());
            decoded[40..46].copy_from_slice(b"level\0");
            let mut packed = Vec::new();
            for value in [64_u32, 85, 1, 0x4000, 4] {
                packed.extend_from_slice(&value.to_le_bytes());
            }
            packed.push(64);
            packed.extend_from_slice(&decoded);
            let mut source = vec![0_u8; 0x300];
            source[0..4].copy_from_slice(&6_u32.to_le_bytes());
            source[0x204..0x208].copy_from_slice(&2_u32.to_le_bytes());
            source[0x208..0x20c].copy_from_slice(&0x12_u32.to_le_bytes());
            source.extend(firmware::invert(&packed));
            fs::write(self.0.join("source.dat"), &source).unwrap();
            fs::write(self.0.join("spec.json"),serde_json::to_vec(&json!({
                "firmware_id":"synthetic","size":source.len(),"sha256":firmware::sha256(&source),
                "version_major":2,"version_minor":version_minor,"anchors":[{"name":"level","value":1348}],
                "windows":windows,"strings":strings
            })).unwrap()).unwrap();
            ["spec.json", "source.dat", "output"]
                .map(|p| self.0.join(p).into_os_string())
                .to_vec()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    #[test]
    fn refuses_version_windows_and_unverified_strings_before_writing() {
        for (minor, windows, strings) in [
            (12, json!([]), json!([])),
            (
                18,
                json!([{"name":"bad","dat_offset":768,"offset":60,"size":8}]),
                json!([]),
            ),
            (18, json!([]), json!([{"address":4136,"value":"wrong"}])),
        ] {
            let fixture = Fixture::new();
            assert!(run(&fixture.arguments(minor, windows, strings)).is_err());
            assert!(!fixture.0.join("output").exists());
        }
    }
    #[test]
    fn analysis_is_reproducible_and_never_overwrites_input_or_output() {
        let fixture = Fixture::new();
        let mut args = fixture.arguments(18, json!([]), json!([{"address":4136,"value":"level"}]));
        let original = fs::read(&args[1]).unwrap();
        run(&args).unwrap();
        let evidence = fs::read(fixture.0.join("output/evidence.generated.json")).unwrap();
        assert!(run(&args).is_err());
        assert_eq!(
            evidence,
            fs::read(fixture.0.join("output/evidence.generated.json")).unwrap()
        );
        args[2] = fixture.0.join("another-output").into_os_string();
        run(&args).unwrap();
        assert_eq!(
            evidence,
            fs::read(fixture.0.join("another-output/evidence.generated.json")).unwrap()
        );
        assert_eq!(original, fs::read(&args[1]).unwrap());
    }
}
