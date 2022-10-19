@external("env", "wasm_input")
declare function wasm_input<T>(x: T): T
export function mix(): i32 {
  var n = wasm_input(0);
  var m = wasm_input(0);
  return n+m;
}
