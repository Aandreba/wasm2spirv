(module
  (type $t0 (func (param i32 i32 i32) (result i32)))
  (func $addTwo (export "addTwo") (type $t0) (param $n i32) (param $p0 i32) (param $p1 i32) (result i32)
    ;; Lhs
    local.get $n
    local.get $p0
    i32.add
    i32.load)
  (memory $memory (export "memory") i32 16)
)
