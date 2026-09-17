import { readFileSync } from 'node:fs';

const config = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8'));
const pkg = JSON.parse(readFileSync('package.json', 'utf8'));
const cargo = readFileSync('src-tauri/Cargo.toml', 'utf8');

const windows = config?.bundle?.windows ?? {};
const webview = windows.webviewInstallMode ?? {};
const cargoVersion = cargo.match(/^version\s*=\s*"([^"]+)"/m)?.[1];

const checks = [
  ['Windows bundles remain enabled', config?.bundle?.active === true && config?.bundle?.targets === 'all'],
  ['WebView2 bootstrapper is embedded for stronger legacy Windows install compatibility', webview.type === 'embedBootstrapper'],
  ['WebView2 bootstrapper runs silently', webview.silent === true],
  ['application identity remains stable', config?.identifier === 'sa.clinicdesk.desktop' && config?.productName === 'ClinicDesk'],
  ['package, Tauri config and Cargo versions match', pkg.version === config.version && cargoVersion === config.version],
  ['Windows icon remains configured', Array.isArray(config?.bundle?.icon) && config.bundle.icon.includes('icons/icon.ico')],
];

const failed = checks.filter(([, ok]) => !ok);
for (const [name, ok] of checks) console.log(`${ok ? 'PASS' : 'FAIL'} ${name}`);
if (failed.length) {
  console.error(`Windows package contract failed: ${failed.length} check(s)`);
  process.exit(1);
}
console.log(`Windows package contract passed: ${checks.length}/${checks.length}`);
