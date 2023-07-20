(module
  (type (;0;) (func (param i32) (result i32)))
  (type (;1;) (func (param i32 i32 i32 i32)))
  (import "spir_global" "gl_GlobalInvocationID" (func (;0;) (type 0)))
  (import "spir_global" "gl_NumWorkGroups" (func (;1;) (type 0)))
  (func (;2;) (type 1) (param i32 i32 i32 i32)
    (local i32 i32 i32 i32 f32 i32)
    i32.const 0
    call 0
    local.tee 4
    i32.const 2
    i32.shl
    local.set 5
    i32.const 0
    call 1
    local.tee 6
    i32.const 2
    i32.shl
    local.set 7
    f32.const 0x0p+0 (;=0;)
    local.set 8
    local.get 4
    local.set 9
    block  ;; label = @1
      loop  ;; label = @2
        local.get 9
        local.get 0
        i32.ge_u
        br_if 1 (;@1;)
        local.get 8
        local.get 1
        local.get 5
        i32.add
        f32.load
        local.get 2
        local.get 5
        i32.add
        f32.load
        f32.mul
        f32.add
        local.set 8
        local.get 5
        local.get 7
        i32.add
        local.set 5
        local.get 9
        local.get 6
        i32.add
        local.set 9
        br 0 (;@2;)
      end
    end
    local.get 3
    local.get 4
    i32.const 2
    i32.shl
    i32.add
    local.get 8
    f32.store)
  (memory (;0;) 16)
  (global (;0;) (mut i32) (i32.const 1048576))
  (export "memory" (memory 0))
  (export "dot" (func 2)))
