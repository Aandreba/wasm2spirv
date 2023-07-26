(module
  (type (;0;) (func (param i32) (result i32)))
  (type (;1;) (func (param i32 i32)))
  (import "spir_global" "gl_GlobalInvocationID" (func (;0;) (type 0)))
  (func (;1;) (type 1) (param i32 i32)
    (local i32)
    local.get 1
    i32.const 0
    call 0
    i32.const 2
    i32.shl
    local.tee 2
    i32.add
    local.get 0
    local.get 2
    i32.add
    i32.load
    local.tee 1
    local.get 1
    i32.mul
    i32.store)
  (memory (;0;) 16)
  (global (;0;) (mut i32) (i32.const 1048576))
  (export "memory" (memory 0))
  (export "Main" (func 1)))
