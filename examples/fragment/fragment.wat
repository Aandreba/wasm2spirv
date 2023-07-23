(module
  (type (;0;) (func (param f32 i32 f32 i32)))
  (func (;0;) (type 0) (param f32 i32 f32 i32)
    block  ;; label = @1
      block  ;; label = @2
        local.get 1
        i32.eqz
        br_if 0 (;@2;)
        local.get 0
        local.get 2
        f32.add
        local.set 0
        br 1 (;@1;)
      end
      local.get 0
      local.get 0
      f32.add
      local.get 2
      f32.div
      local.set 0
    end
    i32.const 4
    local.set 1
    block  ;; label = @1
      loop  ;; label = @2
        local.get 1
        i32.eqz
        br_if 1 (;@1;)
        local.get 1
        i32.const -1
        i32.add
        local.set 1
        local.get 0
        local.get 2
        f32.mul
        local.set 0
        br 0 (;@2;)
      end
    end
    local.get 3
    local.get 0
    f32.store)
  (memory (;0;) 16)
  (global (;0;) (mut i32) (i32.const 1048576))
  (export "memory" (memory 0))
  (export "main" (func 0)))
