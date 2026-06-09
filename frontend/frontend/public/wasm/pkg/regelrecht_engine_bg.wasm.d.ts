/* tslint:disable */
/* eslint-disable */
export const memory: WebAssembly.Memory;
export const __wbg_wasmengine_free: (a: number, b: number) => void;
export const wasmengine_clearDataSources: (a: number) => void;
export const wasmengine_execute: (a: number, b: number, c: number, d: number, e: number, f: any, g: number, h: number) => [number, number, number];
export const wasmengine_executeMultiple: (a: number, b: number, c: number, d: any, e: any, f: number, g: number) => [number, number, number];
export const wasmengine_executeMultipleWithTrace: (a: number, b: number, c: number, d: any, e: any, f: number, g: number) => [number, number, number];
export const wasmengine_executeWithTrace: (a: number, b: number, c: number, d: number, e: number, f: any, g: number, h: number) => [number, number, number];
export const wasmengine_getLawInfo: (a: number, b: number, c: number) => [number, number, number];
export const wasmengine_hasLaw: (a: number, b: number, c: number) => number;
export const wasmengine_lawCount: (a: number) => number;
export const wasmengine_listLaws: (a: number) => [number, number];
export const wasmengine_loadLaw: (a: number, b: number, c: number) => [number, number, number, number];
export const wasmengine_new: () => number;
export const wasmengine_registerDataSource: (a: number, b: number, c: number, d: number, e: number, f: any) => [number, number];
export const wasmengine_resolveNote: (a: number, b: number, c: number, d: any) => [number, number, number];
export const wasmengine_resolveNotes: (a: number, b: number, c: number, d: number, e: number) => [number, number, number];
export const wasmengine_unloadLaw: (a: number, b: number, c: number) => number;
export const wasmengine_version: (a: number) => [number, number];
export const __wbindgen_malloc: (a: number, b: number) => number;
export const __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
export const __wbindgen_exn_store: (a: number) => void;
export const __externref_table_alloc: () => number;
export const __wbindgen_externrefs: WebAssembly.Table;
export const __externref_table_dealloc: (a: number) => void;
export const __externref_drop_slice: (a: number, b: number) => void;
export const __wbindgen_free: (a: number, b: number, c: number) => void;
export const __wbindgen_start: () => void;
