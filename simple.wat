(module
  (type $t0 (func (param i32 i32 i32)))
  (func $addTwo (export "addTwo") (type $t0) (param $n i32) (param $p0 i32) (param $p1 i32)
    local.get $n
    i32.const 2
    i32.shl
    local.tee $n

    ;; Lhs
    local.get $p0
    i32.add
    i32.load

    ;; Rhs
    local.get $n
    local.get $p1
    i32.add
    i32.load

    i32.add
    drop)
  (memory $memory (export "memory") i32 16)
)
