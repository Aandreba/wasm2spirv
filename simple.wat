(module
  (type $t0 (func (param i32 i32) (result i32)))
  (func $addTwo (export "addTwo") (type $t0) (param $p0 i32) (param $p1 i32) (result i32)
    local.get $p0
    i32.load
    local.get $p1
    i32.load
    i32.add)
  (memory $memory (export "memory") i32 16)
)
