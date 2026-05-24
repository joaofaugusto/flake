#!/usr/bin/env node
/**
 * Generates all Tauri icon files from src-tauri/icons/icon.svg
 * Run: npm run generate-icons
 */
import { readFileSync, writeFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const iconsDir   = join(__dirname, '..', 'src-tauri', 'icons');
const svgPath    = join(iconsDir, 'icon.svg');

let sharp, png2icons;
try {
  sharp = (await import('sharp')).default;
} catch {
  console.error('❌  sharp not found — run: npm install --save-dev sharp');
  process.exit(1);
}
try {
  png2icons = (await import('png2icons')).default;
} catch {
  console.error('❌  png2icons not found — run: npm install --save-dev png2icons');
  process.exit(1);
}

const svg = readFileSync(svgPath);
console.log('Generating icons from icon.svg…\n');

// ── PNG files expected by Tauri ────────────────────────────────────────────
const pngTargets = [
  [32,   '32x32.png'],
  [128,  '128x128.png'],
  [256,  '128x128@2x.png'],
  [512,  'icon.png'],
];

for (const [size, name] of pngTargets) {
  await sharp(svg).resize(size, size).png().toFile(join(iconsDir, name));
  console.log(`  ✓  ${name.padEnd(22)} ${size}×${size}`);
}

// ── Windows Store PNGs (required by Tauri bundle) ──────────────────────────
const storeSizes = [
  [30,  'Square30x30Logo.png'],
  [44,  'Square44x44Logo.png'],
  [71,  'Square71x71Logo.png'],
  [89,  'Square89x89Logo.png'],
  [107, 'Square107x107Logo.png'],
  [142, 'Square142x142Logo.png'],
  [150, 'Square150x150Logo.png'],
  [284, 'Square284x284Logo.png'],
  [310, 'Square310x310Logo.png'],
  [50,  'StoreLogo.png'],
];

for (const [size, name] of storeSizes) {
  await sharp(svg).resize(size, size).png().toFile(join(iconsDir, name));
  console.log(`  ✓  ${name.padEnd(22)} ${size}×${size}`);
}

// ── Base 1024 buffer for ico/icns ─────────────────────────────────────────
const base = await sharp(svg).resize(1024, 1024).png().toBuffer();

// .ico — Windows (multi-size)
const ico = png2icons.createICO(base, png2icons.BICUBIC, 0, true);
writeFileSync(join(iconsDir, 'icon.ico'), ico);
console.log('  ✓  icon.ico               (16, 32, 48, 256)');

// .icns — macOS
const icns = png2icons.createICNS(base, png2icons.BICUBIC, 0);
writeFileSync(join(iconsDir, 'icon.icns'), icns);
console.log('  ✓  icon.icns');

console.log('\nAll icons saved to src-tauri/icons/ ✓');
