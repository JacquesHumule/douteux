//! Generates fake warez-release-style names, e.g.
//! `Adobe.Photoshop.v25.3.1.x64.Multilingual.Incl.Keygen-KEYFORGE[A3F9C1D2].rar`
//!
//! Real software product names are used for a convincing disguise — names alone
//! aren't copyrightable/trademark-protected in this kind of nominative,
//! non-commercial reference — but the version, edition, "crack group", etc. are
//! all fictional/generic. The groups in particular are deliberately made up and
//! don't claim to be an actual scene release.

use crate::generators::rng::Rng;

/// Real software products, used purely for the disguise.
const SOFTWARE: &[&str] = &[
    "Adobe Photoshop",
    "Adobe Premiere Pro",
    "Adobe After Effects",
    "Adobe Acrobat Pro",
    "Adobe Illustrator",
    "Adobe Lightroom Classic",
    "Adobe InDesign",
    "Adobe Audition",
    "Autodesk AutoCAD",
    "Autodesk Maya",
    "Autodesk 3ds Max",
    "Autodesk Revit",
    "CorelDRAW Graphics Suite",
    "Corel PaintShop Pro",
    "Sublime Text",
    "JetBrains IntelliJ IDEA",
    "JetBrains PyCharm",
    "JetBrains WebStorm",
    "JetBrains CLion",
    "Microsoft Office",
    "Microsoft Visio",
    "Microsoft Project",
    "WinRAR",
    "Internet Download Manager",
    "IObit Uninstaller Pro",
    "IObit Advanced SystemCare",
    "CCleaner Professional",
    "Ableton Live Suite",
    "FL Studio",
    "Image-Line FL Studio",
    "Steinberg Cubase Pro",
    "PreSonus Studio One",
    "VMware Workstation Pro",
    "Parallels Desktop",
    "Navicat Premium",
    "Cyberlink PowerDirector",
    "Wondershare Filmora",
    "Wondershare Dr.Fone",
    "DAEMON Tools Ultra",
    "Nitro Pro",
    "Malwarebytes Premium",
    "TechSmith Camtasia",
    "TechSmith Snagit",
    "Sketch",
    "Affinity Photo",
    "Affinity Designer",
    "Native Instruments Kontakt",
    "reFX Nexus",
    "Serato DJ Pro",
    "Maxon Cinema 4D",
    "The Foundry Nuke",
    "Bentley MicroStation",
    "SolidWorks",
    "ANSYS",
    "MATLAB",
    "Tableau Desktop",
    "EaseUS Data Recovery Wizard",
    "Sony Vegas Pro",
    "Bitwig Studio",
    "Pinnacle Studio",
];

/// Product editions / build flavours.
const EDITIONS: &[&str] = &[
    "Multilingual",
    "Multilanguage",
    "x64",
    "x86",
    "Win64",
    "RePack",
    "Portable",
    "Preactivated",
    "Retail",
    "LTS",
    "Pro",
    "Enterprise",
    "Full",
];

/// The "how it's cracked" tag.
const CRACK_TAGS: &[&str] = &[
    "Incl.Keygen",
    "Incl.Patch",
    "Incl.Crack",
    "Incl.KeyGen",
    "Cracked",
    "Keygen-only",
    "Patch-only",
    "Activator",
    "Incl.Serial",
    "Regfile",
];

/// Fictional crack groups — inspired by scene naming, but made up.
const GROUPS: &[&str] = &[
    "RAZOR1337",
    "SKIDNET",
    "ZERODAY",
    "HEXWARE",
    "BITLORD",
    "NULLROW",
    "PHANTOMHASP",
    "KEYFORGE",
    "CRACKLAB",
    "VOIDGEN",
    "AMNESIAC",
    "SUDOROOT",
    "PATCHWERK",
    "DONGLEKILL",
    "RIPTIDE",
];

const EXTENSIONS: &[&str] = &["rar", "zip", "7z", "iso", "exe", "msi", "nfo", "sfv", "dmg"];

/// Turn a product name into scene-release-name casing, e.g.
/// `"Adobe Photoshop"` -> `"Adobe.Photoshop"`.
fn normalize_name(name: &str) -> String {
    name.split_whitespace()
        .map(|word| {
            word.chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
        })
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(".")
}

/// Generate a fake warez-style software-crack release name.
pub fn generate() -> String {
    let mut rng = Rng::new();

    let software = normalize_name(rng.pick(SOFTWARE));

    let major = 1 + rng.next_u64() % 30;
    let minor = rng.next_u64() % 20;
    let patch = rng.next_u64() % 20;
    let version = format!("v{major}.{minor}.{patch}");

    let edition = rng.pick(EDITIONS);
    let tag = rng.pick(CRACK_TAGS);
    let group = rng.pick(GROUPS);
    let ext = rng.pick(EXTENSIONS);
    // CRC32-style tag — also keeps slugs from colliding.
    let hash = rng.hex(8);

    format!("{software}.{version}.{edition}.{tag}-{group}[{hash}].{ext}")
}
