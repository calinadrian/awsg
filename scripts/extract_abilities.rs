//! One-off: dump the original's 111 ability definitions from AzerothSurvivors.exe.
//!
//! Usage: extract_abilities <exe> [definitions_rva] [count] [stride]
//!
//! Reads the PE section table, sanity-checks the profile's damage_guard bytes,
//! then decodes each definition's String24 name (u32 cap, u32 size, 24 chars)
//! at offset 16 of a 120-byte record.

use std::env;
use std::fs;
use std::process;

fn le16(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}

fn le32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// RVA -> file offset via the PE section table.
fn rva_to_off(rva: usize, sections: &[(usize, usize, usize)]) -> Option<usize> {
    for &(va, vs, rp) in sections {
        if rva >= va && rva < va + vs {
            return Some(rva - va + rp);
        }
    }
    None
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = args
        .get(1)
        .unwrap_or_else(|| {
            eprintln!("usage: extract_abilities <exe> [rva=112745976] [count=111] [stride=120]");
            process::exit(2);
        });
    let data = fs::read(&path).unwrap_or_else(|e| {
        eprintln!("read {path}: {e}");
        process::exit(1);
    });

    let e_lfanew = le32(&data, 0x3c) as usize;
    assert_eq!(&data[e_lfanew..e_lfanew + 4], b"PE\0\0", "not a PE file");
    let coff = e_lfanew + 4;
    let num_sections = le16(&data, coff + 2) as usize;
    let size_of_optional = le16(&data, coff + 16) as usize;
    let opt = coff + 20;
    let magic = le16(&data, opt);
    let bit64 = magic == 0x20b;
    let section_table = opt + size_of_optional;
    let mut sections: Vec<(usize, usize, usize)> = Vec::new();
    for i in 0..num_sections {
        let s = section_table + i * 40;
        let name = String::from_utf8_lossy(&data[s..s + 8])
            .trim_end_matches('\0')
            .to_string();
        let vsize = le32(&data, s + 8) as usize;
        let vaddr = le32(&data, s + 12) as usize;
        let raw_size = le32(&data, s + 16) as usize;
        let raw_ptr = le32(&data, s + 20) as usize;
        println!(
            "[{i}] {name:<10} va={vaddr:#010x} vsize={vsize:#010x} raw={raw_ptr:#010x} rawsize={raw_size:#010x}"
        );
        sections.push((vaddr, vsize, raw_ptr));
    }
    println!("bit64={bit64} size={}", data.len());

    // Sanity check from the profile: damage_guard.rva 93568280 must read 440fb6be29010000.
    if let Some(o) = rva_to_off(93_568_280, &sections) {
        let bytes = &data[o..o + 8];
        let ok = bytes == [0x44, 0x0f, 0xb6, 0xbe, 0x29, 0x01, 0x00, 0x00];
        println!("damage_guard @93568280 -> {} (expect 440fb6be29010000) {}", hex(bytes), ok);
    } else {
        println!("damage_guard RVA 93568280 not mapped");
    }

    let rva = args
        .get(2)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(112_745_976);
    let count = args.get(3).and_then(|s| s.parse::<usize>().ok()).unwrap_or(111);
    let stride = args.get(4).and_then(|s| s.parse::<usize>().ok()).unwrap_or(120);

    let Some(base) = rva_to_off(rva, &sections) else {
        eprintln!("definitions RVA {rva:#x} not mapped");
        process::exit(1);
    };
    println!("definitions file offset {base:#x}, {count} x {stride} bytes");

    // Full hex of the first three records so the layout is visible.
    for i in 0..3.min(count) {
        let d = base + i * stride;
        println!("--- record {i} hex ---");
        for (c, line) in data[d..d + stride].chunks_exact(16).enumerate() {
            println!("    {d:#x}+{c:02x}  {}", hex(line));
        }
    }

    for i in 0..count {
        let d = base + i * stride;
        if d + stride > data.len() {
            println!("{i:3}: out of bounds");
            break;
        }
        let cap = le32(&data, d + 16);
        let size = le32(&data, d + 20);
        let raw = &data[d + 24..d + 48];
        let name = String::from_utf8_lossy(raw).trim_end_matches('\0').to_string();
        // Definition header (offsets 0..16) as a hint of the id/pointer pattern.
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&data[d..d + 8]);
        let head: u64 = u64::from_le_bytes(buf);
        println!("{i:3}: id={head:#018x} cap={cap} size={size} name={name:?}");
    }
}
