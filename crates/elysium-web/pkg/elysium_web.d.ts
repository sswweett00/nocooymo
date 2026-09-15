/* tslint:disable */
/* eslint-disable */

export class ElysiumWebApp {
    free(): void;
    [Symbol.dispose](): void;
    delete_selected(): void;
    get_fps(): number;
    get_object_count(): number;
    get_scene_json(): string;
    is_playing(): boolean;
    constructor();
    toggle_play(): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_elysiumwebapp_free: (a: number, b: number) => void;
    readonly elysiumwebapp_delete_selected: (a: number) => void;
    readonly elysiumwebapp_get_fps: (a: number) => number;
    readonly elysiumwebapp_get_object_count: (a: number) => number;
    readonly elysiumwebapp_get_scene_json: (a: number) => [number, number];
    readonly elysiumwebapp_is_playing: (a: number) => number;
    readonly elysiumwebapp_new: () => [number, number, number];
    readonly elysiumwebapp_toggle_play: (a: number) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h1cae7eadb2ff40cd: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h1cae7eadb2ff40cd_1: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h1cae7eadb2ff40cd_2: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h1cae7eadb2ff40cd_3: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h386c8fc4eb17ef89: (a: number, b: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_destroy_closure: (a: number, b: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
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
