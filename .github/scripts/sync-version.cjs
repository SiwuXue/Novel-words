const fs = require('node:fs');

const version = process.env.RELEASE_VERSION;

if (!version || !/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(version)) {
  throw new Error(`Invalid release version: ${version ?? '(missing)'}`);
}

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}

function writeJson(file, value) {
  fs.writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`);
}

const packageJson = readJson('package.json');
packageJson.version = version;
writeJson('package.json', packageJson);

const packageLock = readJson('package-lock.json');
packageLock.version = version;
if (packageLock.packages?.['']) {
  packageLock.packages[''].version = version;
}
writeJson('package-lock.json', packageLock);

const tauriConfigPath = 'src-tauri/tauri.conf.json';
const tauriConfig = readJson(tauriConfigPath);
tauriConfig.version = version;
writeJson(tauriConfigPath, tauriConfig);

const cargoPath = 'src-tauri/Cargo.toml';
let cargoToml = fs.readFileSync(cargoPath, 'utf8');
const cargoVersionPattern = /(^\[package\][\s\S]*?^version\s*=\s*")([^"]+)("\s*$)/m;
if (!cargoVersionPattern.test(cargoToml)) {
  throw new Error('Could not find the package version in src-tauri/Cargo.toml');
}
cargoToml = cargoToml.replace(cargoVersionPattern, `$1${version}$3`);
fs.writeFileSync(cargoPath, cargoToml);

console.log(`Synchronized application version to ${version}`);
