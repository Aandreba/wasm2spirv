(module
  (type $t0 (func (param i32) (result i32)))
  (type $t1 (func (param i32 f32 i64 i64)))
  (import "spir_global" "gl_GlobalInvocationID" (func $spir_global.gl_GlobalInvocationID (type $t0)))
  (import "spir_global" "gl_NumWorkGroups" (func $spir_global.gl_NumWorkGroups (type $t0)))
  (func $saxpy (export "saxpy") (type $t1) (param $p0 i32) (param $p1 f32) (param $p2 i64) (param $p3 i64)
    (local $l4 i64) (local $l5 i64) (local $l6 i64) (local $l7 i64) (local $l8 i64) (local $l9 i64)
    i32.const 0
    call $spir_global.gl_GlobalInvocationID
    i64.extend_i32_u
    local.tee $l4
    i64.const 2
    i64.shl
    local.set $l5
    i32.const 0
    call $spir_global.gl_NumWorkGroups
    i64.extend_i32_u
    local.tee $l6
    i64.const 2
    i64.shl
    local.set $l7
    local.get $p0
    i64.extend_i32_u
    local.set $l8
    block $B0
      loop $L1
        local.get $l4
        local.get $l8
        i64.ge_u
        br_if $B0
        local.get $p3
        local.get $l5
        i64.add
        local.tee $l9
        local.get $l9
        f32.load
        local.get $p2
        local.get $l5
        i64.add
        f32.load
        local.get $p1
        f32.mul
        f32.add
        f32.store
        local.get $l5
        local.get $l7
        i64.add
        local.set $l5
        local.get $l4
        local.get $l6
        i64.add
        local.set $l4
        br $L1
      end
    end)
  (memory $memory (export "memory") i64 16)
)
