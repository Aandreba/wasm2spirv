(module
  (type (;0;) (func (param i32) (result i32)))
  (type (;1;) (func (param i32 f32 i32 i32)))
  (import "spir_global" "gl_GlobalInvocationID" (func (;0;) (type 0)))
  (import "spir_global" "gl_NumWorkGroups" (func (;1;) (type 0)))
  (func (;2;) (type 1) (param i32 f32 i32 i32)
    (local i32 i32 i32 i32 i32 i32 f32)
    i32.const 0
    call 0
    local.set 4
    i32.const 0
    call 1
    local.tee 5
    i32.const 2
    i32.shl
    local.set 6
    local.get 2
    local.get 4
    i32.const 2
    i32.shl
    i32.add
    local.set 2
    local.get 5
    i32.const 3
    i32.shl
    local.set 7
    local.get 3
    local.get 4
    i32.const 3
    i32.shl
    i32.add
    local.set 3
    local.get 1
    i32.reinterpret_f32
    local.tee 8
    i32.const 31
    i32.shr_s
    i32.const 1
    i32.shr_u
    local.get 8
    i32.xor
    local.set 9
    block  ;; label = @1
      loop  ;; label = @2
        local.get 4
        local.get 0
        i32.ge_u
        br_if 1 (;@1;)
        local.get 3
        local.get 1
        local.get 2
        f32.load
        local.tee 10
        local.get 9
        local.get 10
        i32.reinterpret_f32
        local.tee 8
        i32.const 31
        i32.shr_s
        i32.const 1
        i32.shr_u
        local.get 8
        i32.xor
        i32.gt_s
        select
        f64.promote_f32
        f64.store
        local.get 2
        local.get 6
        i32.add
        local.set 2
        local.get 3
        local.get 7
        i32.add
        local.set 3
        local.get 4
        local.get 5
        i32.add
        local.set 4
        br 0 (;@2;)
      end
    end)
  (memory (;0;) 16)
  (global (;0;) (mut i32) (i32.const 1048576))
  (export "memory" (memory 0))
  (export "main" (func 2)))
