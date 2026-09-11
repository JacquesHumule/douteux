//! Generates fake "PC optimizer" / freeware-installer names, e.g.
//! `SmartDriverBooster_Pro_Setup_v8.4_x64[A3F9C1D2].exe`
//!
//! The product name is stitched together from generic English word fragments,
//! so any resemblance to a real product (a few combos like "Driver Booster" do
//! exist in the wild) is incidental — and the version, "edition" badge and
//! installer decoration are all fictional/generic.

use crate::generators::rng::Rng;

/// Leading brand-y adjective / qualifier.
const PREFIXES: &[&str] = &[
    "Smart", "Ultra", "Advanced", "Total", "PC", "Max", "Turbo", "Easy", "Super", "Power", "Win",
    "My", "Magic", "Wise", "Auto", "Rapid", "Perfect", "Active", "Cyber", "One",
];

/// What the tool claims to work on (optional middle word).
const TARGETS: &[&str] = &[
    "Driver", "Registry", "Disk", "System", "RAM", "Startup", "Privacy", "Junk", "Malware",
    "Network", "PC", "Web",
];

/// The "what it does" noun.
const NOUNS: &[&str] = &[
    "Cleaner",
    "Optimizer",
    "Booster",
    "Updater",
    "Defender",
    "Guard",
    "Doctor",
    "Fixer",
    "Mechanic",
    "Care",
    "TuneUp",
    "Repair",
    "Manager",
    "Toolkit",
    "Utilities",
    "Master",
    "Genius",
    "Wizard",
    "Scanner",
    "Shield",
    "Accelerator",
    "Assistant",
];

/// Edition / marketing badge.
const BADGES: &[&str] = &[
    "Pro", "Plus", "Premium", "Ultimate", "Gold", "Deluxe", "FREE", "Advanced", "Elite", "2024",
    "2025",
];

/// Installer flavour.
const INSTALLERS: &[&str] = &[
    "Setup",
    "Installer",
    "Web_Setup",
    "Full_Setup",
    "Portable",
    "Downloader",
    "Install",
];

const ARCHS: &[&str] = &["x64", "x86", "Win64", "Win32", "Multilingual"];

const EXTENSIONS: &[&str] = &["exe", "msi", "zip", "dmg", "iso"];

/// Generate a fake "PC optimizer" / freeware-installer name.
pub fn generate() -> String {
    let mut rng = Rng::new();

    let mut name = String::from(*rng.pick(PREFIXES));
    // Sometimes name the thing it claims to fix, e.g. "SmartDriverBooster".
    if rng.next_u64() % 2 == 0 {
        name.push_str(rng.pick(TARGETS));
    }
    name.push_str(rng.pick(NOUNS));

    let major = 1 + rng.next_u64() % 20;
    let minor = rng.next_u64() % 10;

    let badge = rng.pick(BADGES);
    let installer = rng.pick(INSTALLERS);
    let arch = rng.pick(ARCHS);
    let ext = rng.pick(EXTENSIONS);
    // Build-tag — also keeps slugs from colliding.
    let hash = rng.hex(8);

    format!("{name}_{badge}_{installer}_v{major}.{minor}_{arch}[{hash}].{ext}")
}
