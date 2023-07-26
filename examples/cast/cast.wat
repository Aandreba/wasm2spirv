(module
  (type $t0 (func (param i32 f32 i32 i32)))
  (func $main (export "main") (type $t0) (param $p0 i32) (param $p1 f32) (param $p2 i32) (param $p3 i32)
    local.get $p3
    local.get $p0
    i32.const 3
    i32.shl
    i32.add
    local.get $p1
    local.get $p2
    local.get $p0
    i32.const 2
    i32.shl
    i32.add
    i32.load
    local.tee $p0
    f32.reinterpret_i32
    local.get $p1
    i32.reinterpret_f32
    local.tee $p2
    i32.const 31
    i32.shr_s
    i32.const 1
    i32.shr_u
    local.get $p2
    i32.xor
    local.get $p0
    i32.const 31
    i32.shr_s
    i32.const 1
    i32.shr_u
    local.get $p0
    i32.xor
    i32.gt_s
    select
    f64.promote_f32
    f64.store)
  (memory $memory (export "memory") 16)
  (global $g0 (mut i32) (i32.const 1048576)))
