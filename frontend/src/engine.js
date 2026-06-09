/**
 * In-browser regelrecht engine (WASM) with the NAPP law corpus pre-loaded.
 *
 * The WASM package is built via `just wasm` into public/wasm/pkg/. The law
 * YAML files are bundled as raw strings at build time, so the scenario runner
 * works without backend.
 */

import wppYaml from '../../law/wet_op_de_politieke_partijen/2026-01-01.yaml?raw';
import regelingYaml from '../../law/regeling_subsidiebedragen/2026-01-01.yaml?raw';
import awbYaml from '../../law/algemene_wet_bestuursrecht/1994-01-01.yaml?raw';

let enginePromise = null;

export function getEngine() {
  if (enginePromise) return enginePromise;
  enginePromise = (async () => {
    const jsRes = await fetch('/wasm/pkg/regelrecht_engine.js');
    if (!jsRes.ok) {
      throw new Error(
        'WASM-engine niet gevonden. Bouw hem eerst met `just wasm`.',
      );
    }
    const jsText = await jsRes.text();
    const blob = new Blob([jsText], { type: 'application/javascript' });
    const blobUrl = URL.createObjectURL(blob);
    const wasm = await import(/* @vite-ignore */ blobUrl);
    URL.revokeObjectURL(blobUrl);
    await wasm.default('/wasm/pkg/regelrecht_engine_bg.wasm');
    const engine = new wasm.WasmEngine();
    engine.loadLaw(wppYaml);
    engine.loadLaw(regelingYaml);
    engine.loadLaw(awbYaml);
    return engine;
  })();
  return enginePromise;
}
