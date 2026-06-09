/* tslint:disable */
/* eslint-disable */

/**
 * WASM-compatible law execution engine with cross-law resolution.
 *
 * Backed by `LawExecutionService`, providing automatic resolution of
 * cross-law references and data source support in the browser.
 */
export class WasmEngine {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Remove all registered data sources.
     */
    clearDataSources(): void;
    /**
     * Execute a law output with automatic cross-law resolution.
     *
     * All referenced laws must be loaded via `loadLaw()` first.
     * Data sources registered via `registerDataSource()` are queried
     * to resolve inputs before falling back to cross-law resolution.
     *
     * # Arguments
     * * `law_id` - ID of the loaded law
     * * `output_name` - Name of the output to calculate
     * * `parameters` - JavaScript object with input parameters
     * * `calculation_date` - Date string (YYYY-MM-DD) for which to calculate
     *
     * # Returns
     * * `Ok(JsValue)` - JavaScript object with `outputs`, `resolved_inputs`, etc.
     * * `Err(JsValue)` - Error message if execution fails
     */
    execute(law_id: string, output_name: string, parameters: any, calculation_date: string): any;
    /**
     * Execute multiple specific outputs from a law (privacy-by-design).
     *
     * Callers must explicitly list which outputs they need. The engine evaluates
     * each producing article once and returns only the requested outputs.
     *
     * # Arguments
     * * `law_id` - ID of the loaded law
     * * `output_names` - JavaScript array of output name strings
     * * `parameters` - JavaScript object with input parameters
     * * `calculation_date` - Date string (YYYY-MM-DD)
     */
    executeMultiple(law_id: string, output_names: any, parameters: any, calculation_date: string): any;
    /**
     * Execute multiple specific outputs with tracing enabled.
     */
    executeMultipleWithTrace(law_id: string, output_names: any, parameters: any, calculation_date: string): any;
    /**
     * Execute a law output with tracing enabled.
     *
     * Same as `execute()` but includes a full execution trace tree in the
     * result. The trace captures every resolution step, cross-law call, and
     * operation performed during evaluation.
     *
     * # Returns
     * * `Ok(JsValue)` - JavaScript object with `outputs`, `trace` (tree), `trace_text` (box-drawing)
     * * `Err(JsValue)` - Error message if execution fails (may include partial trace)
     */
    executeWithTrace(law_id: string, output_name: string, parameters: any, calculation_date: string): any;
    /**
     * Get metadata about a loaded law.
     */
    getLawInfo(law_id: string): any;
    /**
     * Check if a law is loaded.
     */
    hasLaw(law_id: string): boolean;
    /**
     * Get the number of loaded laws.
     */
    lawCount(): number;
    /**
     * List all loaded law IDs (sorted alphabetically).
     */
    listLaws(): string[];
    /**
     * Load a law from a YAML string.
     *
     * If a law with the same ID and valid_from is already loaded, it will be replaced.
     * Multiple versions (same ID, different valid_from) can coexist.
     *
     * # Arguments
     * * `yaml` - YAML string containing the law definition (max 1 MB)
     *
     * # Returns
     * * `Ok(String)` - The law ID
     * * `Err(JsValue)` - Error message if parsing fails
     */
    loadLaw(yaml: string): string;
    /**
     * Create a new empty engine instance.
     */
    constructor();
    /**
     * Register a tabular data source from flat records.
     *
     * Data sources are queried during execution to resolve inputs before
     * falling back to cross-law resolution.
     *
     * # Arguments
     * * `name` - Data source name (e.g., "personal_data")
     * * `key_field` - Field name used as record key (e.g., "bsn")
     * * `records` - JavaScript array of objects, each representing a record
     *
     * # Example (JavaScript)
     * ```javascript
     * engine.registerDataSource('personal_data', 'bsn', [
     *     { bsn: '999993653', geboortedatum: '2000-01-01', land_verblijf: 'NEDERLAND' }
     * ]);
     * ```
     */
    registerDataSource(name: string, key_field: string, records: any): void;
    /**
     * Resolve a single TextQuoteSelector against a loaded law (RFC-005).
     *
     * The law must already be loaded via `loadLaw()`. The selector is
     * content-addressed, so it resolves on whichever version is loaded.
     *
     * # Arguments
     * * `law_id` - ID of the loaded law
     * * `selector` - JS object: `{ exact, prefix?, suffix?, "regelrecht:hint"? }`
     *
     * # Returns
     * * `Ok(JsValue)` - `MatchResult`: `{ status, matches: [{ article_number,
     *   start, end, confidence, matched_text }] }`. `start`/`end` are **`char`
     *   offsets** (Unicode scalar values), not UTF-16 code units: JS code
     *   slicing the article text must convert accordingly.
     * * `Err(JsValue)` - Error if the law is not loaded or the selector is invalid
     */
    resolveNote(law_id: string, selector: any): any;
    /**
     * Resolve all notes in a note-sidecar YAML string against a loaded law.
     *
     * Parses an `annotations:` document (the format validated by
     * `just validate-annotations`) and resolves every note's selector.
     *
     * Notes whose `target.source` names a different law than `law_id` are
     * skipped (a sidecar may legitimately carry notes for several laws). A
     * note with a missing or unparseable selector is **not** silently
     * dropped: it appears in the result with an `error` string and a `null`
     * `match`, so the caller can distinguish "no notes" from "notes that
     * failed to parse".
     *
     * # Arguments
     * * `law_id` - ID of the loaded law to resolve against
     * * `annotations_yaml` - Contents of an `annotations.yaml` sidecar file
     *
     * # Returns
     * * `Ok(JsValue)` - Array of `{ note, match, error }` objects. `match` is
     *   the `MatchResult` (or `null` on error); `error` is `null` on success
     *   or a message string. Match `start`/`end` are **`char` offsets**, not
     *   UTF-16 code units.
     * * `Err(JsValue)` - Error if the law is not loaded or the YAML is invalid
     */
    resolveNotes(law_id: string, annotations_yaml: string): any;
    /**
     * Remove a loaded law from the engine.
     *
     * # Returns
     * * `true` if the law was removed, `false` if it wasn't loaded
     */
    unloadLaw(law_id: string): boolean;
    /**
     * Get the engine version.
     */
    version(): string;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_wasmengine_free: (a: number, b: number) => void;
    readonly wasmengine_clearDataSources: (a: number) => void;
    readonly wasmengine_execute: (a: number, b: number, c: number, d: number, e: number, f: any, g: number, h: number) => [number, number, number];
    readonly wasmengine_executeMultiple: (a: number, b: number, c: number, d: any, e: any, f: number, g: number) => [number, number, number];
    readonly wasmengine_executeMultipleWithTrace: (a: number, b: number, c: number, d: any, e: any, f: number, g: number) => [number, number, number];
    readonly wasmengine_executeWithTrace: (a: number, b: number, c: number, d: number, e: number, f: any, g: number, h: number) => [number, number, number];
    readonly wasmengine_getLawInfo: (a: number, b: number, c: number) => [number, number, number];
    readonly wasmengine_hasLaw: (a: number, b: number, c: number) => number;
    readonly wasmengine_lawCount: (a: number) => number;
    readonly wasmengine_listLaws: (a: number) => [number, number];
    readonly wasmengine_loadLaw: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmengine_new: () => number;
    readonly wasmengine_registerDataSource: (a: number, b: number, c: number, d: number, e: number, f: any) => [number, number];
    readonly wasmengine_resolveNote: (a: number, b: number, c: number, d: any) => [number, number, number];
    readonly wasmengine_resolveNotes: (a: number, b: number, c: number, d: number, e: number) => [number, number, number];
    readonly wasmengine_unloadLaw: (a: number, b: number, c: number) => number;
    readonly wasmengine_version: (a: number) => [number, number];
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __externref_drop_slice: (a: number, b: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
